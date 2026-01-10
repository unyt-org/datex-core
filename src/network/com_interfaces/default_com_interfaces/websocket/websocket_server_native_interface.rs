use crate::std_sync::Mutex;
use crate::stdlib::{
    collections::HashMap, future::Future, net::SocketAddr, pin::Pin,
};
use core::prelude::rust_2024::*;
use core::result::Result;
use core::time::Duration;
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::{
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

use futures_util::stream::SplitSink;
use tokio_tungstenite::accept_async;

use super::websocket_common::{
    WebSocketError, WebSocketServerError, WebSocketServerInterfaceSetupData,
    parse_url,
};
use crate::runtime::global_context::{get_global_context, set_global_context};
use tokio_tungstenite::WebSocketStream;
use crate::network::com_interfaces::com_interface::implementation::ComInterfaceImplementation;
use crate::network::com_interfaces::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::ComInterfaceFactory;
use crate::network::com_interfaces::com_interface::properties::{InterfaceDirection, InterfaceProperties};
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;

type WebsocketStreamMap = HashMap<
    ComInterfaceSocketUUID,
    SplitSink<WebSocketStream<TcpStream>, Message>,
>;

pub struct WebSocketServerNativeInterface {
    pub address: Url,
    websocket_streams: Arc<Mutex<WebsocketStreamMap>>,
    shutdown_signal: Arc<Notify>,
    handle: RefCell<Option<JoinHandle<()>>>,
    com_interface: Rc<RefCell<ComInterface>>,
}

impl WebSocketServerNativeInterface {
    pub fn new(
        port: u16,
        secure: bool,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<WebSocketServerNativeInterface, WebSocketServerError> {
        let address: String = format!("0.0.0.0:{port}");
        let address = parse_url(&address, secure).map_err(|_| {
            WebSocketServerError::WebSocketError(WebSocketError::InvalidURL)
        })?;
        let interface = WebSocketServerNativeInterface {
            address,
            websocket_streams: Arc::new(Mutex::new(HashMap::new())),
            shutdown_signal: Arc::new(Notify::new()),
            handle: RefCell::new(None),
            com_interface,
        };
        Ok(interface)
    }

    async fn open(&self) -> Result<(), WebSocketServerError> {
        let address = self.address.clone();
        info!("Spinning up server at {address}");
        let addr = format!(
            "{}:{}",
            address.host_str().unwrap(),
            address.port_or_known_default().unwrap()
        )
        .parse::<SocketAddr>()
        .map_err(|_| WebSocketServerError::InvalidPort)?;

        let listener = TcpListener::bind(&addr).await.map_err(|_| {
            WebSocketServerError::WebSocketError(
                WebSocketError::ConnectionError,
            )
        })?;

        let websocket_streams = self.websocket_streams.clone();
        let shutdown = self.shutdown_signal.clone();
        let mut tasks: Vec<JoinHandle<()>> = vec![];
        let global_context = get_global_context();

        let manager = self
            .com_interface
            .borrow()
            .socket_manager();

        self.handle.replace(Some(spawn(async move {
            let global_context = global_context.clone();
            let manager = manager.clone();
            set_global_context(global_context.clone());
            info!("WebSocket server started at {addr}");
            loop {
                let manager = manager.clone();
                select! {
                    res = listener.accept() => {
                        match res {
                            Ok((stream, addr)) => {
                                let websocket_streams = websocket_streams.clone();
                                let global_context = global_context.clone();
                                info!("New connection from {addr}");
                                let task = spawn(async move {
                                    set_global_context(global_context.clone());
                                    let manager = manager.clone();

                                    match accept_async(stream).await {
                                        Ok(ws_stream) => {
                                            info!(
                                                "Accepted WebSocket connection from {addr}"
                                            );
                                            let (write, mut read) = ws_stream.split();

                                            let (socket_uuid, mut sender) = manager
                                                .lock()
                                                .unwrap()
                                                .create_and_init_socket(InterfaceDirection::InOut, 1);

                                            websocket_streams
                                                .try_lock()
                                                .unwrap()
                                                .insert(socket_uuid.clone(), write);

                                            while let Some(msg) = read.next().await {
                                                match msg {
                                                    Ok(Message::Binary(bin)) => {
                                                        sender.start_send(bin).unwrap();
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
                                                    .try_lock()
                                                    .unwrap();
                                            streams.remove(&socket_uuid);

                                            manager
                                                .lock()
                                                .unwrap()
                                                .remove_socket(socket_uuid);
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
        })));
        Ok(())
    }
}

impl ComInterfaceFactory
    for WebSocketServerNativeInterface
{
    type SetupData = WebSocketServerInterfaceSetupData;

    fn create(
        setup_data: Self::SetupData,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<WebSocketServerNativeInterface, ComInterfaceError> {
        WebSocketServerNativeInterface::new(
            setup_data.port,
            setup_data.secure.unwrap_or(true),
            com_interface,
        )
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

impl ComInterfaceImplementation for WebSocketServerNativeInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx = self.websocket_streams.clone();
        Box::pin(async move {
            let tx = &mut tx.try_lock().unwrap();
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

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            name: Some(self.address.to_string()),
            ..Self::get_default_properties()
        }
    }

    fn handle_close<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let shutdown_signal = self.shutdown_signal.clone();
        let websocket_streams = self.websocket_streams.clone();
        Box::pin(async move {
            shutdown_signal.notify_waiters();
            if let Some(handle) = self.handle.borrow_mut().take() {
                let _ = handle.await;
            }
            websocket_streams.try_lock().unwrap().clear();
            true
        })
    }

    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output=bool> + 'a>> {
        Box::pin(async move { self.open().await.is_ok() })
    }
}
