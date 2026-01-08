use crate::network::com_interfaces::socket_provider::MultipleSocketProvider;
use crate::std_sync::Mutex;
use crate::stdlib::collections::HashMap;
use crate::stdlib::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use crate::stdlib::pin::Pin;
use crate::stdlib::sync::Arc;
use crate::task::UnboundedSender;
use crate::task::spawn_with_panic_notify_default;
use core::future::Future;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::time::Duration;
use datex_macros::{com_interface, create_opener};
use log::{error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use super::tcp_common::{TCPError, TCPServerInterfaceSetupData};
use crate::network::com_interfaces::com_interface_old::{
    ComInterfaceOld, ComInterfaceError, ComInterfaceFactoryOld, ComInterfaceState,
};
use crate::network::com_interfaces::com_interface_old::{
    ComInterfaceInfo, ComInterfaceSockets,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::{delegate_com_interface_info, set_opener};

pub struct TCPServerNativeInterface {
    pub address: SocketAddr,
    tx: Arc<Mutex<HashMap<ComInterfaceSocketUUID, Arc<Mutex<OwnedWriteHalf>>>>>,
    info: ComInterfaceInfo,
}

impl MultipleSocketProvider for TCPServerNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}

#[com_interface]
impl TCPServerNativeInterface {
    pub fn new(port: u16) -> Result<TCPServerNativeInterface, TCPError> {
        let info = ComInterfaceInfo::new();
        let address =
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port));
        let interface = TCPServerNativeInterface {
            address,
            info,
            tx: Arc::new(Mutex::new(HashMap::new())),
        };
        Ok(interface)
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), TCPError> {
        let address = self.address;
        info!("Spinning up server at {address}");

        let listener = TcpListener::bind(self.address)
            .await
            .map_err(|e| TCPError::Other(format!("{e:?}")))?;
        info!("Server listening on {address}");

        let interface_uuid = self.uuid().clone();
        let sockets = self.get_sockets().clone();
        let tx = self.tx.clone();
        // TODO #615: use normal spawn (thread)? currently leads to global context panic
        spawn_with_panic_notify_default(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let socket = ComInterfaceSocket::init(
                            interface_uuid.clone(),
                            InterfaceDirection::InOut,
                            1,
                        );
                        let (read_half, write_half) = stream.into_split();
                        tx.try_lock().unwrap().insert(
                            socket.uuid.clone(),
                            Arc::new(Mutex::new(write_half)),
                        );

                        let bytes_in_sender = socket.bytes_in_sender.clone();
                        sockets
                            .try_lock()
                            .unwrap()
                            .add_socket(Arc::new(Mutex::new(socket)));

                        spawn_with_panic_notify_default(async move {
                            Self::handle_client(read_half, bytes_in_sender)
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
    }

    async fn handle_client(
        mut rx: OwnedReadHalf,
        bytes_in_sender: Arc<Mutex<UnboundedSender<Vec<u8>>>>,
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
                    let mut queue = bytes_in_sender.try_lock().unwrap();
                    queue.start_send(buffer[..n].to_vec());
                }
                Err(e) => {
                    error!("Failed to read from socket: {e}");
                    break;
                }
            }
        }
    }
}

impl ComInterfaceFactoryOld<TCPServerInterfaceSetupData>
    for TCPServerNativeInterface
{
    fn create(
        setup_data: TCPServerInterfaceSetupData,
    ) -> Result<TCPServerNativeInterface, ComInterfaceError> {
        TCPServerNativeInterface::new(setup_data.port)
            .map_err(|_| ComInterfaceError::InvalidSetupData)
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

impl ComInterfaceOld for TCPServerNativeInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx_queues = self.tx.clone();
        let tx_queues = tx_queues.try_lock().unwrap();
        let tx = tx_queues.get(&socket);
        if tx.is_none() {
            error!("Client is not connected");
            return Box::pin(async { false });
        }
        let tx = tx.unwrap().clone();
        Box::pin(
            async move { tx.try_lock().unwrap().write(block).await.is_ok() },
        )
    }
    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            name: Some(self.address.to_string()),
            ..Self::get_default_properties()
        }
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO #207
        Box::pin(async move { true })
    }

    delegate_com_interface_info!();
    set_opener!(open);
}
