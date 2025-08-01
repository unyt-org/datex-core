use std::time::Duration;
use std::{
    collections::HashMap, future::Future, net::SocketAddr, pin::Pin,
    sync::Mutex,
};
// FIXME no-std

use crate::network::com_interfaces::socket_provider::MultipleSocketProvider;
use crate::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{ComInterface, ComInterfaceInfo, ComInterfaceSockets},
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
    },
    set_opener,
    stdlib::sync::Arc,
    task::spawn,
};
use datex_macros::{com_interface, create_opener};

use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    sync::Notify,
    task::JoinHandle,
};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceFactory, ComInterfaceState,
};
use futures_util::stream::SplitSink;
use tokio_tungstenite::accept_async;

use super::websocket_common::{
    WebSocketError, WebSocketServerError, WebSocketServerInterfaceSetupData,
    parse_url,
};
use crate::runtime::global_context::{get_global_context, set_global_context};
use tokio_tungstenite::WebSocketStream;

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
    handle: Option<JoinHandle<()>>,
}

impl MultipleSocketProvider for WebSocketServerNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}

#[com_interface]
impl WebSocketServerNativeInterface {
    pub fn new(
        port: u16,
    ) -> Result<WebSocketServerNativeInterface, WebSocketServerError> {
        let address: String = format!("0.0.0.0:{port}");
        let address = parse_url(&address).map_err(|_| {
            WebSocketServerError::WebSocketError(WebSocketError::InvalidURL)
        })?;
        let interface = WebSocketServerNativeInterface {
            address,
            info: ComInterfaceInfo::new(),
            websocket_streams: Arc::new(Mutex::new(HashMap::new())),
            shutdown_signal: Arc::new(Notify::new()),
            handle: None,
        };
        Ok(interface)
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), WebSocketServerError> {
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
        let shutdown = self.shutdown_signal.clone();
        let mut tasks: Vec<JoinHandle<()>> = vec![];
        let global_context = get_global_context();
        self.handle = Some(spawn(async move {
            let global_context = global_context.clone();
            set_global_context(global_context.clone());
            info!("WebSocket server started at {addr}");
            loop {
                select! {
                    res = listener.accept() => {
                        match res {
                            Ok((stream, addr)) => {
                                let websocket_streams = websocket_streams.clone();
                                let interface_uuid = interface_uuid.clone();
                                let com_interface_sockets = com_interface_sockets.clone();
                                let global_context = global_context.clone();
                                info!("New connection from {addr}");
                                let task = spawn(async move {
                                    set_global_context(global_context.clone());

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
                                                .insert(socket_uuid.clone(), write);

                                            while let Some(msg) = read.next().await {
                                                match msg {
                                                    Ok(Message::Binary(bin)) => {
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
                                            // consider the connection closed, clean up
                                            let mut streams =
                                                websocket_streams
                                                    .lock()
                                                    .unwrap();
                                            streams.remove(&socket_uuid);
                                            com_interface_sockets
                                                .lock()
                                                .unwrap()
                                                .remove_socket(&socket_uuid);
                                            info!(
                                                "WebSocket connection from {addr} closed"
                                            );
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
        }));
        Ok(())
    }
}

impl ComInterfaceFactory<WebSocketServerInterfaceSetupData>
    for WebSocketServerNativeInterface
{
    fn create(
        setup_data: WebSocketServerInterfaceSetupData,
    ) -> Result<WebSocketServerNativeInterface, ComInterfaceError> {
        WebSocketServerNativeInterface::new(setup_data.port)
            .map_err(|_| ComInterfaceError::InvalidSetupData)
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
        let shutdown_signal = self.shutdown_signal.clone();
        let websocket_streams = self.websocket_streams.clone();
        Box::pin(async move {
            shutdown_signal.notify_waiters();
            if let Some(handle) = self.handle.take() {
                let _ = handle.await;
            }
            websocket_streams.lock().unwrap().clear();
            true
        })
    }
    delegate_com_interface_info!();
    set_opener!(open);
}
