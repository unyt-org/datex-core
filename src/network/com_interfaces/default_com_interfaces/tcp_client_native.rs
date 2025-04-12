use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedWriteHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::mpsc;
use url::Url;

use crate::network::com_interfaces::com_interface::{
    ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::socket_provider::SingleSocketProvider;
use crate::network::com_interfaces::tcp::tcp_common::TCPError;
use crate::utils::uuid::UUID;

use super::super::com_interface::ComInterface;

pub struct TCPClientNativeInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    tx: Option<Arc<Mutex<OwnedWriteHalf>>>,
}
impl SingleSocketProvider for TCPClientNativeInterface {
    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
}

impl TCPClientNativeInterface {
    pub async fn open(
        address: &str,
    ) -> Result<TCPClientNativeInterface, TCPError> {
        let uuid = ComInterfaceUUID(UUID::new());
        let mut interface = TCPClientNativeInterface {
            address: Url::parse(address).map_err(|_| TCPError::InvalidURL)?,
            com_interface_sockets: Arc::new(Mutex::new(
                ComInterfaceSockets::default(),
            )),
            uuid,
            tx: None,
        };
        interface.start().await?;
        Ok(interface)
    }

    async fn start(&mut self) -> Result<(), TCPError> {
        let address = self.address.clone();
        info!(
            "Connecting to WebSocket server at {}",
            address.host_str().unwrap()
        );
        let stream = TcpStream::connect(address.to_string())
            .await
            .map_err(|_| TCPError::ConnectionError)?;

        let (read_half, write_half) = stream.into_split();
        self.tx = Some(Arc::new(Mutex::new(write_half)));

        let socket = ComInterfaceSocket::new(
            self.uuid.clone(),
            InterfaceDirection::IN_OUT,
            1,
        );
        let receive_queue = socket.receive_queue.clone();
        self.com_interface_sockets
            .lock()
            .unwrap()
            .add_socket(Arc::new(Mutex::new(socket)));
        spawn(async move {
            let mut reader = read_half;
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        warn!("Connection closed by peer");
                        break;
                    }
                    Ok(n) => {
                        info!("Received: {:?}", &buffer[..n]);
                        receive_queue.lock().unwrap().extend(buffer);
                    }
                    Err(e) => {
                        error!("Failed to read from socket: {}", e);
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}

impl ComInterface for TCPClientNativeInterface {
    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "tcp".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
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

    fn get_uuid(&self) -> &ComInterfaceUUID {
        &self.uuid
    }

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
}
