use std::{
    collections::HashMap, future::Future, net::SocketAddr, pin::Pin,
    sync::Mutex,
}; // FIXME no-std

use crate::{
    network::com_interfaces::{
        com_interface::{ComInterface, ComInterfaceSockets, ComInterfaceUUID},
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
        websocket::websocket_common::{WebSocketError, WebSocketServerError},
    },
    stdlib::sync::Arc,
    utils::uuid::UUID,
};

use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::websocket::websocket_common::parse_url;
use futures_util::stream::SplitSink;
use tokio_tungstenite::accept_async;

use tokio_tungstenite::WebSocketStream;

pub struct WebSocketServerNativeInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    websocket_streams: Arc<
        Mutex<
            HashMap<
                ComInterfaceSocketUUID,
                SplitSink<WebSocketStream<TcpStream>, Message>,
            >,
        >,
    >,
}

impl WebSocketServerNativeInterface {
    pub async fn open(
        port: &u16,
    ) -> Result<WebSocketServerNativeInterface, WebSocketServerError> {
        let address: String = format!("127.0.0.1:{}", port);
        let address = parse_url(&address).map_err(|_| {
            WebSocketServerError::WebSocketError(WebSocketError::InvalidURL)
        })?;
        let mut interface = WebSocketServerNativeInterface {
            address,
            uuid: ComInterfaceUUID(UUID::new()),
            com_interface_sockets: Arc::new(Mutex::new(
                ComInterfaceSockets::default(),
            )),
            websocket_streams: Arc::new(Mutex::new(HashMap::new())),
        };
        interface.start().await.map_err(|_| {
            WebSocketServerError::WebSocketError(
                WebSocketError::ConnectionError,
            )
        })?;
        Ok(interface)
    }

    async fn start(&mut self) -> Result<(), WebSocketServerError> {
        let address = self.address.clone();
        info!("Spinning up server at {}", address);
        let addr = format!(
            "{}:{}",
            address.host_str().unwrap(),
            address.port().unwrap()
        )
        .parse::<SocketAddr>()
        .map_err(|_| WebSocketServerError::InvalidPort)?;

        let listener = TcpListener::bind(&addr).await.map_err(|_| {
            WebSocketServerError::WebSocketError(
                WebSocketError::ConnectionError,
            )
        })?;
        let interface_uuid = self.uuid.clone();
        let com_interface_sockets = self.com_interface_sockets.clone();
        let websocket_streams = self.websocket_streams.clone();
        tokio::spawn(async move {
            loop {
                let (stream, addr) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
                let websocket_streams = websocket_streams.clone();
                let interface_uuid = interface_uuid.clone();
                let com_interface_sockets = com_interface_sockets.clone();
                tokio::spawn(async move {
                    match accept_async(stream).await {
                        Ok(ws_stream) => {
                            info!(
                                "Accepted WebSocket connection from {}",
                                addr
                            );
                            let (write, mut read) = ws_stream.split();
                            let socket = ComInterfaceSocket::new(
                                interface_uuid.clone(),
                                InterfaceDirection::IN_OUT,
                                1,
                            );
                            let socket_uuid = socket.uuid.clone();
                            let socket_shared = Arc::new(Mutex::new(socket));

                            com_interface_sockets
                                .clone()
                                .lock()
                                .unwrap()
                                .add_socket(socket_shared.clone());

                            websocket_streams
                                .lock()
                                .unwrap()
                                .insert(socket_uuid, write);

                            while let Some(msg) = read.next().await {
                                match msg {
                                    Ok(Message::Binary(bin)) => {
                                        debug!(
                                            "Received binary message: {:?}",
                                            bin
                                        );
                                        socket_shared
                                            .lock()
                                            .unwrap()
                                            .receive_queue
                                            .lock()
                                            .unwrap()
                                            .extend(bin);
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!(
                                            "WebSocket error from {}: {}",
                                            addr, e
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "WebSocket handshake failed with {}: {}",
                                addr, e
                            );
                        }
                    }
                });
            }
        });
        Ok(())
    }
}

impl ComInterface for WebSocketServerNativeInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx = self.websocket_streams.clone();
        Box::pin(async move {
            let tx = &mut tx.lock().unwrap();
            let tx = tx.get_mut(&socket_uuid);
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
            round_trip_time: std::time::Duration::from_millis(40),
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
