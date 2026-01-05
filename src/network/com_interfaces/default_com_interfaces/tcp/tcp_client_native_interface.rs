use super::tcp_common::{TCPClientInterfaceSetupData, TCPError};
use crate::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceError, ComInterfaceFactory, ComInterfaceState,
};
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::socket_provider::SingleSocketProvider;
use crate::std_sync::Mutex;
use crate::stdlib::net::SocketAddr;
use crate::stdlib::pin::Pin;
use crate::stdlib::rc::Rc;
use crate::stdlib::sync::Arc;
use crate::task::spawn;
use crate::{delegate_com_interface_info, set_opener};
use core::cell::RefCell;
use core::future::Future;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::str::FromStr;
use core::time::Duration;
use datex_macros::{com_interface, create_opener};
use log::{error, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;

pub struct TCPClientNativeInterface {
    pub address: SocketAddr,
    tx: Option<Rc<RefCell<OwnedWriteHalf>>>,
    info: ComInterfaceInfo,
}
impl SingleSocketProvider for TCPClientNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

#[com_interface]
impl TCPClientNativeInterface {
    pub fn new(address: &str) -> Result<TCPClientNativeInterface, TCPError> {
        let interface = TCPClientNativeInterface {
            address: SocketAddr::from_str(address)
                .map_err(|_| TCPError::InvalidAddress)?,
            info: ComInterfaceInfo::new(),
            tx: None,
        };
        Ok(interface)
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), TCPError> {
        let stream = TcpStream::connect(self.address)
            .await
            .map_err(|_| TCPError::ConnectionError)?;

        let (read_half, write_half) = stream.into_split();
        self.tx = Some(Rc::new(RefCell::new(write_half)));

        let socket = ComInterfaceSocket::init(
            self.get_uuid().clone(),
            InterfaceDirection::InOut,
            1,
        );
        let receive_queue = socket.receive_queue.clone();
        self.get_sockets()
            .try_lock()
            .unwrap()
            .add_socket(Arc::new(Mutex::new(socket)));

        let state = self.get_info().state.clone();
        spawn(async move {
            let mut reader = read_half;
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        warn!("Connection closed by peer");
                        state
                            .try_lock()
                            .unwrap()
                            .set(ComInterfaceState::Destroyed);
                        break;
                    }
                    Ok(n) => {
                        let mut queue = receive_queue.try_lock().unwrap();
                        queue.extend(&buffer[..n]);
                    }
                    Err(e) => {
                        error!("Failed to read from socket: {e}");
                        state
                            .try_lock()
                            .unwrap()
                            .set(ComInterfaceState::Destroyed);
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}

impl ComInterface for TCPClientNativeInterface {
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
        Box::pin(
            async move { tx.unwrap().borrow_mut().write(block).await.is_ok() },
        )
    }
    fn init_properties(&self) -> InterfaceProperties {
        Self::get_default_properties()
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO #208
        Box::pin(async move { true })
    }

    delegate_com_interface_info!();
    set_opener!(open);
}

impl ComInterfaceFactory<TCPClientInterfaceSetupData>
    for TCPClientNativeInterface
{
    fn create(
        setup_data: TCPClientInterfaceSetupData,
    ) -> Result<TCPClientNativeInterface, ComInterfaceError> {
        TCPClientNativeInterface::new(&setup_data.address)
            .map_err(|_| ComInterfaceError::InvalidSetupData)
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "tcp-client".to_string(),
            channel: "tcp".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
}
