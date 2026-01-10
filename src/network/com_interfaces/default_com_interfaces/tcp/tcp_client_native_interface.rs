use super::tcp_common::{TCPClientInterfaceSetupData, TCPError};

use crate::network::com_interfaces::com_interface::implementation::{
    ComInterfaceFactory, ComInterfaceImplementation,
};
use crate::network::com_interfaces::com_interface::properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface::socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::com_interface::state::ComInterfaceState;
use crate::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceInfo,
};
use crate::std_sync::Mutex;
use crate::stdlib::net::SocketAddr;
use crate::stdlib::pin::Pin;
use crate::stdlib::rc::Rc;
use crate::stdlib::sync::Arc;
use crate::task::spawn;
use core::cell::RefCell;
use core::future::Future;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::str::FromStr;
use core::time::Duration;
use futures_util::FutureExt;
use datex_macros::{com_interface, create_opener};
use log::{error, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;

pub struct TCPClientNativeInterface {
    pub address: SocketAddr,
    tx: Rc<RefCell<Option<OwnedWriteHalf>>>,
    com_interface: Rc<ComInterface>,
}

impl TCPClientNativeInterface {
    async fn open(&self) -> Result<(), TCPError> {
        let stream = TcpStream::connect(self.address)
            .await
            .map_err(|_| TCPError::ConnectionError)?;

        let (read_half, write_half) = stream.into_split();

        let (_, mut sender) = self
            .com_interface
            .borrow()
            .socket_manager()
            .lock()
            .unwrap()
            .create_and_init_socket(InterfaceDirection::InOut, 1);
        self.tx.borrow_mut().replace(write_half);

        let state = self.com_interface.state();

        spawn(async move {
            let mut reader = read_half;
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        warn!("Connection closed by peer");
                        state.lock().unwrap().set(ComInterfaceState::Destroyed);
                        break;
                    }
                    Ok(n) => {
                        sender.start_send(buffer[..n].to_vec()).unwrap();
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

impl ComInterfaceImplementation for TCPClientNativeInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx = self.tx.clone();
        Box::pin(async move {
            let mut tx = tx.borrow_mut();
            if let Some(tx) = tx.as_mut() {
                match tx.write_all(block).await {
                    Ok(_) => true,
                    Err(e) => {
                        error!("Failed to send data: {}", e);
                        false
                    }
                }
            } else {
                error!("Client is not connected");
                false
            }
        })
    }
    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "tcp-client".to_string(),
            channel: "tcp".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    fn handle_close<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO #208
        Box::pin(async move { true })
    }

    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { self.open().await.is_ok() })
    }
}

impl ComInterfaceFactory for TCPClientNativeInterface {
    type SetupData = TCPClientInterfaceSetupData;

    fn create(
        setup_data: Self::SetupData,
        com_interface: Rc<ComInterface>,
    ) -> Result<
        Self,
        ComInterfaceError,
    > {
        let address = SocketAddr::from_str(&setup_data.address)
            .map_err(|_| ComInterfaceError::InvalidSetupData)?;
        Ok(TCPClientNativeInterface {
            address,
            tx: Rc::new(RefCell::new(None)),
            com_interface,
        })
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
