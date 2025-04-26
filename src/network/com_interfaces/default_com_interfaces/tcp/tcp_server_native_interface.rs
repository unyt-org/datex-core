use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::task::spawn;
use log::{error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use url::Url;

use crate::network::com_interfaces::com_interface::{ComInterface, ComInterfaceError, ComInterfaceFactory, ComInterfaceState};
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::{delegate_com_interface, delegate_com_interface_info, set_opener};
use crate::network::com_interfaces::default_com_interfaces::tcp::tcp_client_native_interface::TCPClientNativeInterface;
use super::tcp_common::{TCPClientInterfaceSetupData, TCPError, TCPServerInterfaceSetupData};

pub struct TCPServerNativeInterface {
    pub address: Url,
    tx: Arc<Mutex<HashMap<ComInterfaceSocketUUID, Arc<Mutex<OwnedWriteHalf>>>>>,
    info: ComInterfaceInfo,
}

impl TCPServerNativeInterface {
    delegate_com_interface!();
    pub fn new(port: u16) -> Result<TCPServerNativeInterface, TCPError> {
        let info = ComInterfaceInfo::new();
        let address: String = format!("ws://127.0.0.1:{port}");
        let address = Url::parse(&address).map_err(|_| TCPError::InvalidURL)?;
        let interface = TCPServerNativeInterface {
            address,
            info,
            tx: Arc::new(Mutex::new(HashMap::new())),
        };
        Ok(interface)
    }

    pub async fn open(&mut self) -> Result<(), TCPError> {
        self.set_state(ComInterfaceState::Connecting);
        let res = {
            let address = self.address.clone();
            info!("Spinning up server at {address}");

            let host = self.address.host_str().ok_or(TCPError::InvalidURL)?;
            let port = self.address.port().ok_or(TCPError::InvalidURL)?;
            let address = format!("{host}:{port}");

            let listener = TcpListener::bind(address.clone())
                .await
                .map_err(|e| TCPError::Other(format!("{e:?}")))?;
            info!("Server listening on {address}");

            let interface_uuid = self.get_uuid().clone();
            let sockets = self.get_sockets().clone();
            let tx = self.tx.clone();
            spawn(async move {
                loop {
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            let socket = ComInterfaceSocket::new(
                                interface_uuid.clone(),
                                InterfaceDirection::InOut,
                                1,
                            );
                            let (read_half, write_half) = stream.into_split();
                            tx.lock().unwrap().insert(
                                socket.uuid.clone(),
                                Arc::new(Mutex::new(write_half)),
                            );

                            let receive_queue = socket.receive_queue.clone();
                            sockets
                                .lock()
                                .unwrap()
                                .add_socket(Arc::new(Mutex::new(socket)));

                            spawn(async move {
                                Self::handle_client(read_half, receive_queue)
                                    .await
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {e}");
                            continue;
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

    async fn handle_client(
        mut rx: OwnedReadHalf,
        receive_queue: Arc<Mutex<VecDeque<u8>>>,
    ) {
        let mut buffer = [0u8; 1024];
        loop {
            match rx.read(&mut buffer).await {
                Ok(0) => {
                    warn!("Connection closed by peer");
                    break;
                }
                Ok(n) => {
                    info!("Received: {:?}", &buffer[..n]);
                    let mut queue = receive_queue.lock().unwrap();
                    queue.extend(&buffer[..n]);
                }
                Err(e) => {
                    error!("Failed to read from socket: {e}");
                    break;
                }
            }
        }
    }
}

impl ComInterfaceFactory<TCPServerInterfaceSetupData> for TCPServerNativeInterface {
    fn create(
        setup_data: TCPServerInterfaceSetupData,
    ) -> Result<TCPServerNativeInterface, ComInterfaceError> {
        TCPServerNativeInterface::new(setup_data.port).map_err(|_|
            ComInterfaceError::InvalidSetupData
        )
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "tcp-server".to_string(),
            channel: "tcp".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
}

impl ComInterface for TCPServerNativeInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx_queues = self.tx.clone();
        let tx_queues = tx_queues.lock().unwrap();
        let tx = tx_queues.get(&socket);
        if tx.is_none() {
            error!("Client is not connected");
            return Box::pin(async { false });
        }
        let tx = tx.unwrap().clone();
        Box::pin(async move { tx.lock().unwrap().write(block).await.is_ok() })
    }
    fn init_properties(&self) -> InterfaceProperties {
        Self::get_default_properties()
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO
        Box::pin(async move { true })
    }

    delegate_com_interface_info!();
    set_opener!(open);
}
