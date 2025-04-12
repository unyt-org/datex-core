use std::{future::Future, pin::Pin, sync::Mutex, time::Duration}; // FIXME no-std

use crate::{
    network::com_interfaces::{
        com_interface::{ComInterface, ComInterfaceSockets, ComInterfaceUUID},
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
        socket_provider::SingleSocketProvider,
        websocket::websocket_common::WebSocketError,
    },
    stdlib::sync::Arc,
    utils::uuid::UUID,
};

use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::{net::TcpStream, spawn};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::websocket::websocket_common::parse_url;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub struct WebSocketClientNativeInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    websocket_stream:
        Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
}

impl SingleSocketProvider for WebSocketClientNativeInterface {
    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
}

impl WebSocketClientNativeInterface {
    pub async fn open(
        address: &str,
    ) -> Result<WebSocketClientNativeInterface, WebSocketError> {
        let address =
            parse_url(address).map_err(|_| WebSocketError::InvalidURL)?;
        let uuid = ComInterfaceUUID(UUID::new());
        let com_interface_sockets =
            Arc::new(Mutex::new(ComInterfaceSockets::default()));
        let mut interface = WebSocketClientNativeInterface {
            address,
            uuid,
            com_interface_sockets,
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
            self.uuid.clone(),
            InterfaceDirection::IN_OUT,
            1,
        );
        self.websocket_stream = Some(write);
        let receive_queue = socket.receive_queue.clone();
        self.com_interface_sockets
            .lock()
            .unwrap()
            .add_socket(Arc::new(Mutex::new(socket)));

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

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "websocket".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    fn get_uuid(&self) -> &ComInterfaceUUID {
        &self.uuid
    }

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
}
