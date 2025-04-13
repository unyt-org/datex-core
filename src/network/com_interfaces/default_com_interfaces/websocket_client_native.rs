use std::{future::Future, pin::Pin, sync::Mutex, time::Duration}; // FIXME no-std

use crate::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceInfo, ComInterfaceSockets,
            ComInterfaceUUID,
        },
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
        socket_provider::SingleSocketProvider,
        websocket::websocket_common::WebSocketError,
    },
    stdlib::sync::Arc,
};

use crate::network::com_interfaces::com_interface::ComInterfaceState;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::{net::TcpStream, spawn};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::websocket::websocket_common::parse_url;
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
    pub async fn open(
        address: &str,
    ) -> Result<WebSocketClientNativeInterface, WebSocketError> {
        let address =
            parse_url(address).map_err(|_| WebSocketError::InvalidURL)?;
        let info = ComInterfaceInfo::new();
        let mut interface = WebSocketClientNativeInterface {
            address,
            info,
            websocket_stream: None,
        };
        interface.start().await?;
        Ok(interface)
    }

    async fn start(&mut self) -> Result<(), WebSocketError> {
        let address = self.address.clone();
        info!(
            "Connecting to WebSocket server at {}",
            address.host_str().unwrap()
        );
        let (stream, _) = tokio_tungstenite::connect_async(address)
            .await
            .map_err(|_| WebSocketError::ConnectionError)?;
        let (write, mut read) = stream.split();
        let socket = ComInterfaceSocket::new(
            self.get_uuid().clone(),
            InterfaceDirection::IN_OUT,
            1,
        );
        self.websocket_stream = Some(write);
        let receive_queue = socket.receive_queue.clone();
        self.get_sockets()
            .lock()
            .unwrap()
            .add_socket(Arc::new(Mutex::new(socket)));

        self.set_state(ComInterfaceState::Connected);
        let state = self.get_info().get_state();
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
                        error!("WebSocket read error: {}", e);
                        state
                            .lock()
                            .unwrap()
                            .set_state(ComInterfaceState::Closed);
                        break;
                    }
                }
            }
        });
        Ok(())
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
            debug!("Sending block: {:?}", block);
            tx.unwrap()
                .send(Message::Binary(block.to_vec()))
                .await
                .map_err(|e| {
                    error!("Error sending message: {:?}", e);
                    false
                })
                .is_ok()
        })
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "websocket".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    delegate_com_interface_info!();
}
