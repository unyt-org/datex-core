use crate::std_sync::Mutex;
use crate::stdlib::{future::Future, pin::Pin, time::Duration};
use core::prelude::rust_2024::*;
use core::result::Result;
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use log::{debug, error, info};
use tokio::net::TcpStream;
use tungstenite::Message;
use url::Url;

use super::websocket_common::{
    WebSocketClientInterfaceSetupData, WebSocketError, parse_url,
};
use crate::task::spawn_with_panic_notify_default;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use crate::network::com_interfaces::com_interface::implementation::ComInterfaceImplementation;
use crate::network::com_interfaces::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::ComInterfaceFactory;
use crate::network::com_interfaces::com_interface::properties::{InterfaceDirection, InterfaceProperties};
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::com_interface::state::ComInterfaceState;

pub struct WebSocketClientNativeInterface {
    pub address: Url,
    websocket_stream:
        RefCell<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    com_interface: Rc<RefCell<ComInterface>>,
}
impl WebSocketClientNativeInterface {
    pub fn new(
        address: &str,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<WebSocketClientNativeInterface, WebSocketError> {
        let address =
            parse_url(address, true).map_err(|_| WebSocketError::InvalidURL)?;
        let interface = WebSocketClientNativeInterface {
            address,
            com_interface,
            websocket_stream: RefCell::new(None),
        };
        Ok(interface)
    }

    async fn open(&self) -> Result<(), WebSocketError> {
        let address = self.address.clone();
        info!("Connecting to WebSocket server at {address}");
        let (stream, _) = tokio_tungstenite::connect_async(address)
            .await
            .map_err(|e| {
                error!("Failed to connect to WebSocket server: {e}");
                WebSocketError::ConnectionError
            })?;
        let (write, mut read) = stream.split();

        self.websocket_stream.replace(Some(write));

        let (_, mut sender) = self
            .com_interface
            .borrow()
            .socket_manager()
            .lock()
            .unwrap()
            .create_and_init_socket(InterfaceDirection::InOut, 1);

        let state = self.com_interface.borrow().state();
        
        spawn_with_panic_notify_default(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        sender.start_send(data).unwrap();
                    }
                    Ok(_) => {
                        error!("Invalid message type received");
                    }
                    Err(e) => {
                        error!("WebSocket read error: {e}");
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

impl ComInterfaceFactory
    for WebSocketClientNativeInterface
{

    type SetupData = WebSocketClientInterfaceSetupData;

    fn create(
        setup_data: Self::SetupData,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<WebSocketClientNativeInterface, ComInterfaceError> {
        WebSocketClientNativeInterface::new(&setup_data.address, com_interface)
            .map_err(|_| ComInterfaceError::InvalidSetupData)
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "websocket-client".to_string(),
            channel: "websocket".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
}

impl ComInterfaceImplementation for WebSocketClientNativeInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move {
            // TODO: no borrow across await
            let mut websocket_stream = self.websocket_stream.borrow_mut();
            let tx = websocket_stream.as_mut();
            match tx {
                Some(tx) => {
                    debug!("Sending block: {block:?}");
                    tx
                        .send(Message::Binary(block.to_vec()))
                        .await
                        .map_err(|e| {
                            error!("Error sending message: {e:?}");
                            false
                        })
                        .is_ok()
                }
                None => {
                    error!("WebSocket client is not connected");
                    false
                }
            }
        })
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            name: Some(self.address.to_string()),
            ..Self::get_default_properties()
        }
    }

    fn handle_close<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO #210
        Box::pin(async move { true })
    }

    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output=bool> + 'a>> {
        Box::pin(async move { self.open().await.is_ok() })
    }
}
