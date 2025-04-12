use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
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
use crate::network::com_interfaces::tcp::tcp_common::TCPError;
use crate::utils::uuid::UUID;

use super::super::com_interface::ComInterface;

pub struct TCPServerNativeInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    tx: Arc<Mutex<HashMap<ComInterfaceSocketUUID, Arc<Mutex<OwnedWriteHalf>>>>>,
}

impl TCPServerNativeInterface {
    pub async fn open(
        port: &u16,
    ) -> Result<TCPServerNativeInterface, TCPError> {
        let uuid: ComInterfaceUUID = ComInterfaceUUID(UUID::new());
        let address: String = format!("ws://127.0.0.1:{}", port);
        let address = Url::parse(&address).map_err(|_| TCPError::InvalidURL)?;

        let mut interface = TCPServerNativeInterface {
            address,
            com_interface_sockets: Arc::new(Mutex::new(
                ComInterfaceSockets::default(),
            )),
            uuid,
            tx: Arc::new(Mutex::new(HashMap::new())),
        };
        interface.start().await?;
        Ok(interface)
    }

    async fn start(&mut self) -> Result<(), TCPError> {
        let address = self.address.clone();
        info!("Spinning up server at {}", address);

        let host = self.address.host_str().ok_or(TCPError::InvalidURL)?;
        let port = self.address.port().ok_or(TCPError::InvalidURL)?;
        let address = format!("{}:{}", host, port);

        let listener = TcpListener::bind(address.clone())
            .await
            .map_err(|e| TCPError::Other(format!("{:?}", e)))?;
        info!("Server listening on {}", address);

        let interface_uuid = self.uuid.clone();
        let sockets = self.com_interface_sockets.clone();
        let tx = self.tx.clone();
        spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let socket = ComInterfaceSocket::new(
                            interface_uuid.clone(),
                            InterfaceDirection::IN_OUT,
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
                            Self::handle_client(read_half, receive_queue).await
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        continue;
                    }
                }
            }
        });
        Ok(())
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
                    error!("Failed to read from socket: {}", e);
                    break;
                }
            }
        }
    }
}

impl ComInterface for TCPServerNativeInterface {
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

    fn get_uuid(&self) -> &ComInterfaceUUID {
        &self.uuid
    }

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
}
