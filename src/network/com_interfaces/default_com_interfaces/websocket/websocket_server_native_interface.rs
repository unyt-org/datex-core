use std::{
    collections::HashMap, future::Future, net::SocketAddr, pin::Pin,
    sync::Mutex,
};
use std::time::Duration;
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
    },
    set_opener,
    stdlib::sync::Arc,
    task::spawn,
};

use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    sync::Notify,
    task::JoinHandle,
};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::com_interface::{ComInterfaceError, ComInterfaceFactory, ComInterfaceState};
use futures_util::stream::SplitSink;
use tokio_tungstenite::accept_async;

use tokio_tungstenite::WebSocketStream;
use super::websocket_common::{parse_url, WebSocketError, WebSocketServerError, WebSocketServerInterfaceSetupData};

pub struct WebSocketServerNativeInterface {
    pub address: Url,
    websocket_streams: Arc<
        Mutex<
            HashMap<
                ComInterfaceSocketUUID,
                SplitSink<WebSocketStream<TcpStream>, Message>,
            >,
        >,
    >,
    info: ComInterfaceInfo,
    shutdown_signal: Arc<Notify>,
}

impl WebSocketServerNativeInterface {
    delegate_com_interface!();
    pub fn new(
        port: u16,
    ) -> Result<WebSocketServerNativeInterface, WebSocketServerError> {
        let address: String = format!("127.0.0.1:{port}");
        let address = parse_url(&address).map_err(|_| {
            WebSocketServerError::WebSocketError(WebSocketError::InvalidURL)
        })?;
        let interface = WebSocketServerNativeInterface {
            address,
            info: ComInterfaceInfo::new(),
            websocket_streams: Arc::new(Mutex::new(HashMap::new())),
            shutdown_signal: Arc::new(Notify::new()),
        };
        Ok(interface)
    }

    pub async fn open(&mut self) -> Result<(), WebSocketServerError> {
        let res = {
            let address = self.address.clone();
            info!("Spinning up server at {address}");
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

            let interface_uuid = self.get_uuid().clone();
            let com_interface_sockets = self.get_sockets().clone();
            let websocket_streams = self.websocket_streams.clone();
            self.set_state(ComInterfaceState::Connected);
            let shutdown = self.shutdown_signal.clone();
            let mut tasks: Vec<JoinHandle<()>> = vec![];

            spawn(async move {
                loop {
                    debug!("ipdating...");
                    select! {
                        res = listener.accept() => {
                            match res {
                                Ok((stream, addr)) => {
                                    let websocket_streams = websocket_streams.clone();
                                    let interface_uuid = interface_uuid.clone();
                                    let com_interface_sockets = com_interface_sockets.clone();
                                    let task = spawn(async move {
                                        match accept_async(stream).await {
                                            Ok(ws_stream) => {
                                                info!(
                                                    "Accepted WebSocket connection from {addr}"
                                                );
                                                let (write, mut read) = ws_stream.split();
                                                let socket = ComInterfaceSocket::new(
                                                    interface_uuid.clone(),
                                                    InterfaceDirection::InOut,
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
                                                                "Received binary message: {bin:?}"
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
                                                                "WebSocket error from {addr}: {e}"
                                                            );
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!(
                                                    "WebSocket handshake failed with {addr}: {e}"
                                                );
                                            }
                                        }
                                    });
                                    tasks.push(task);
                                }
                                Err(e) => {
                                    error!("Failed to accept connection: {e}");
                                    continue;
                                }
                            };
                        }
                        _ = shutdown.notified() => {
                            info!("Shutdown signal received, stopping server...");
                            for task in tasks {
                                task.abort();
                            }
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

impl ComInterfaceFactory<WebSocketServerInterfaceSetupData> for WebSocketServerNativeInterface {
    fn create(
        setup_data: WebSocketServerInterfaceSetupData,
    ) -> Result<WebSocketServerNativeInterface, ComInterfaceError> {
        WebSocketServerNativeInterface::new(setup_data.port).map_err(|_|
            ComInterfaceError::InvalidSetupData
        )
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "websocket-server".to_string(),
            channel: "websocket".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
}

impl ComInterface for WebSocketServerNativeInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx = self.websocket_streams.clone();
        Box::pin(async move {
            let tx = &mut tx.lock().unwrap();
            let tx = tx.get_mut(&socket_uuid);
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
        WebSocketServerNativeInterface::get_default_properties()
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let shutdown_signal = self.shutdown_signal.clone();
        let websocket_streams = self.websocket_streams.clone();
        Box::pin(async move {
            debug!("fire");
            shutdown_signal.notified().await;
            debug!("fire d");

            websocket_streams.lock().unwrap().clear();
            true
        })
    }
    delegate_com_interface_info!();
    set_opener!(open);
}
