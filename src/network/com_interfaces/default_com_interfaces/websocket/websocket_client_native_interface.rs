use crate::std_sync::Mutex;
use crate::stdlib::{future::Future, pin::Pin, time::Duration};
use core::prelude::rust_2024::*;
use core::result::Result;

use crate::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface_old::{ComInterfaceOld, ComInterfaceInfo, ComInterfaceSockets},
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
        socket_provider::SingleSocketProvider,
    },
    set_opener,
    stdlib::sync::Arc,
};
use datex_macros::{com_interface, create_opener};

use crate::network::com_interfaces::com_interface_old::{
    ComInterfaceError, ComInterfaceFactoryOld, ComInterfaceState,
};
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

#[derive(Debug)]
pub struct WebSocketClientNativeInterface {
    pub address: Url,
    websocket_stream:
        Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    info: ComInterfaceInfo,
}

impl SingleSocketProvider for WebSocketClientNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

#[com_interface]
impl WebSocketClientNativeInterface {
    pub fn new(
        address: &str,
    ) -> Result<WebSocketClientNativeInterface, WebSocketError> {
        let address =
            parse_url(address, true).map_err(|_| WebSocketError::InvalidURL)?;
        let info = ComInterfaceInfo::new();
        let interface = WebSocketClientNativeInterface {
            address,
            info,
            websocket_stream: None,
        };
        Ok(interface)
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), WebSocketError> {
        let address = self.address.clone();
        info!("Connecting to WebSocket server at {address}");
        let (stream, _) = tokio_tungstenite::connect_async(address)
            .await
            .map_err(|e| {
                error!("Failed to connect to WebSocket server: {e}");
                WebSocketError::ConnectionError
            })?;
        let (write, mut read) = stream.split();
        let socket = ComInterfaceSocket::init(
            self.uuid().clone(),
            InterfaceDirection::InOut,
            1,
        );
        self.websocket_stream = Some(write);
        let bytes_in_sender = socket.bytes_in_sender.clone();
        self.get_sockets()
            .try_lock()
            .unwrap()
            .add_socket(Arc::new(Mutex::new(socket)));
        let state = self.get_info().state.clone();
        spawn_with_panic_notify_default(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        let mut queue = bytes_in_sender.try_lock().unwrap();
                        queue.start_send(data);
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

impl ComInterfaceFactoryOld<WebSocketClientInterfaceSetupData>
    for WebSocketClientNativeInterface
{
    fn create(
        setup_data: WebSocketClientInterfaceSetupData,
    ) -> Result<WebSocketClientNativeInterface, ComInterfaceError> {
        WebSocketClientNativeInterface::new(&setup_data.address)
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

impl ComInterfaceOld for WebSocketClientNativeInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move {
            let tx = self.websocket_stream.as_mut();
            if tx.is_none() {
                error!("Client is not connected");
                return false;
            }
            debug!("Sending block: {block:?}");
            tx.unwrap()
                .send(Message::Binary(block.to_vec()))
                .await
                .map_err(|e| {
                    error!("Error sending message: {e:?}");
                    false
                })
                .is_ok()
        })
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
        // TODO #210
        Box::pin(async move { true })
    }

    delegate_com_interface_info!();
    set_opener!(open);
}
