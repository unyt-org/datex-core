use std::{future::Future, pin::Pin, sync::Mutex, time::Duration};
// FIXME no-std

use crate::{
    delegate_com_interface, delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceInfo, ComInterfaceSockets,
            ComInterfaceUUID,
        },
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
        socket_provider::SingleSocketProvider,
    },
    set_opener,
    stdlib::sync::Arc,
    task::spawn,
};

use crate::network::com_interfaces::com_interface::{ComInterfaceError, ComInterfaceFactory, ComInterfaceState};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::net::TcpStream;
use tungstenite::Message;
use url::Url;

use super::websocket_common::{parse_url, WebSocketClientInterfaceSetupData, WebSocketError};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

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

impl WebSocketClientNativeInterface {
    delegate_com_interface!();
    pub fn new(
        address: &str,
    ) -> Result<WebSocketClientNativeInterface, WebSocketError> {
        let address =
            parse_url(address).map_err(|_| WebSocketError::InvalidURL)?;
        let info = ComInterfaceInfo::new();
        let interface = WebSocketClientNativeInterface {
            address,
            info,
            websocket_stream: None,
        };
        Ok(interface)
    }

    pub async fn open(&mut self) -> Result<(), WebSocketError> {
        let res = {
            let address = self.address.clone();
            info!("Connecting to WebSocket server at {address}");
            let (stream, _) = tokio_tungstenite::connect_async(address)
                .await
                .map_err(|_| WebSocketError::ConnectionError)?;
            let (write, mut read) = stream.split();
            let socket = ComInterfaceSocket::new(
                self.get_uuid().clone(),
                InterfaceDirection::InOut,
                1,
            );
            self.websocket_stream = Some(write);
            let receive_queue = socket.receive_queue.clone();
            self.get_sockets()
                .lock()
                .unwrap()
                .add_socket(Arc::new(Mutex::new(socket)));

            self.set_state(ComInterfaceState::Connected);
            let state = self.get_info().state.clone();
            spawn(async move {
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Binary(data)) => {
                            let mut queue = receive_queue.lock().unwrap();
                            queue.extend(data);
                        }
                        Ok(_) => {
                            error!("Invalid message type received");
                        }
                        Err(e) => {
                            error!("WebSocket read error: {e}");
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

impl ComInterfaceFactory<WebSocketClientInterfaceSetupData> for WebSocketClientNativeInterface {
    fn create(
        setup_data: WebSocketClientInterfaceSetupData,
    ) -> Result<WebSocketClientNativeInterface, ComInterfaceError> {
        WebSocketClientNativeInterface::new(&setup_data.address).map_err(|_|
            ComInterfaceError::InvalidSetupData
        )
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

impl ComInterface for WebSocketClientNativeInterface {
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
        WebSocketClientNativeInterface::get_default_properties()
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
