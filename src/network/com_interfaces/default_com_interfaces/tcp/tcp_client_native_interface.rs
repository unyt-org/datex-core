use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceState,
};
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::socket_provider::SingleSocketProvider;
use crate::task::spawn;
use crate::{delegate_com_interface, delegate_com_interface_info, set_opener};
use log::{error, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use url::Url;

use super::tcp_common::TCPError;

pub struct TCPClientNativeInterface {
    pub address: Url,
    tx: Option<Arc<Mutex<OwnedWriteHalf>>>,
    info: ComInterfaceInfo,
}
impl SingleSocketProvider for TCPClientNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

impl TCPClientNativeInterface {
    delegate_com_interface!();
    pub fn new(address: &str) -> Result<TCPClientNativeInterface, TCPError> {
        let interface = TCPClientNativeInterface {
            address: Url::parse(address).map_err(|_| TCPError::InvalidURL)?,
            info: ComInterfaceInfo::new(),
            tx: None,
        };
        Ok(interface)
    }

    pub async fn open(&mut self) -> Result<(), TCPError> {
        self.set_state(ComInterfaceState::Connecting);

        let res = {
            let host = self.address.host_str().ok_or(TCPError::InvalidURL)?;
            let port = self.address.port().ok_or(TCPError::InvalidURL)?;
            let address = format!("{host}:{port}");
            let stream = TcpStream::connect(address)
                .await
                .map_err(|_| TCPError::ConnectionError)?;

            let (read_half, write_half) = stream.into_split();
            self.tx = Some(Arc::new(Mutex::new(write_half)));

            let socket = ComInterfaceSocket::new(
                self.get_uuid().clone(),
                InterfaceDirection::InOut,
                1,
            );
            let receive_queue = socket.receive_queue.clone();
            self.get_sockets()
                .lock()
                .unwrap()
                .add_socket(Arc::new(Mutex::new(socket)));

            self.set_state(ComInterfaceState::Connected);
            let state = self.get_info().state.clone();
            spawn(async move {
                let mut reader = read_half;
                let mut buffer = [0u8; 1024];
                loop {
                    match reader.read(&mut buffer).await {
                        Ok(0) => {
                            warn!("Connection closed by peer");
                            state
                                .lock()
                                .unwrap()
                                .set(ComInterfaceState::Destroyed);
                            break;
                        }
                        Ok(n) => {
                            let mut queue = receive_queue.lock().unwrap();
                            queue.extend(&buffer[..n]);
                        }
                        Err(e) => {
                            error!("Failed to read from socket: {e}");
                            state
                                .lock()
                                .unwrap()
                                .set(ComInterfaceState::Destroyed);
                            break;
                        }
                    }
                }
            });
            Ok(())
        };

        if res.is_ok() {
            self.set_state(ComInterfaceState::Connected);
        } else {
            self.set_state(ComInterfaceState::NotConnected);
        }
        res
    }
}

impl ComInterface for TCPClientNativeInterface {
    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "tcp".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO
        Box::pin(async move { true })
    }
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx = self.tx.clone();
        if tx.is_none() {
            error!("Client is not connected");
            return Box::pin(async { false });
        }
        Box::pin(async move {
            tx.unwrap().lock().unwrap().write(block).await.is_ok()
        })
    }

    delegate_com_interface_info!();
    set_opener!(open);
}
