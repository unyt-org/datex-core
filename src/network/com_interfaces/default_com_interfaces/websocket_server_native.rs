use std::{
    collections::HashMap, future::Future, net::SocketAddr, pin::Pin,
    sync::Mutex,
}; // FIXME no-std

use crate::{
    network::com_interfaces::websocket::{
        websocket_common::WebSocketError,
        websocket_server::{WebSocketServerError, WebSocketServerInterface},
    },
    stdlib::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc},
};

use futures_util::StreamExt;
use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::websocket::{
    websocket_common::parse_url, websocket_server::WebSocket,
};
use futures_util::stream::SplitSink;
use tokio_tungstenite::accept_async;

use tokio_tungstenite::WebSocketStream;
pub struct WebSocketServerNative {
    pub tx_streams: Arc<
        Mutex<
            HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>>,
        >,
    >,
    address: Url,
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
}

impl WebSocketServerNative {
    fn new(
        address: &str,
    ) -> Result<WebSocketServerNative, WebSocketServerError> {
        let address = parse_url(address).map_err(|_| {
            WebSocketServerError::WebSocketError(WebSocketError::InvalidURL)
        })?;
        Ok(WebSocketServerNative {
            tx_streams: Arc::new(Mutex::new(HashMap::new())),
            receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            address,
        })
    }
}

impl WebSocket for WebSocketServerNative {
    fn connect<'a>(
        &'a mut self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Arc<Mutex<VecDeque<u8>>>,
                        WebSocketServerError,
                    >,
                > + 'a,
        >,
    > {
        let address = self.get_address();
        let receive_queue = self.receive_queue.clone();
        Box::pin(async move {
            info!(
                "Connecting to WebSocket server at {}",
                address.host_str().unwrap()
            );
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

            let queue_clone = receive_queue.clone();
            let clients = self.tx_streams.clone(); // Arc<Mutex<HashMap<SocketAddr, Sender<Message>>>>

            tokio::spawn(async move {
                loop {
                    let (stream, addr) = match listener.accept().await {
                        Ok(pair) => pair,
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                            continue;
                        }
                    };

                    let queue = queue_clone.clone();
                    // let clients = self.clients.clone();
                    let clients_map = clients.clone();

                    tokio::spawn(async move {
                        match accept_async(stream).await {
                            Ok(ws_stream) => {
                                info!(
                                    "Accepted WebSocket connection from {}",
                                    addr
                                );
                                let (write, mut read) = ws_stream.split();
                                // self
                                // clients.insert(addr, write);
                                clients_map.lock().unwrap().insert(addr, write);

                                // self.clients
                                while let Some(msg) = read.next().await {
                                    match msg {
                                        Ok(Message::Binary(bin)) => {
                                            debug!(
                                                "Received binary message: {:?}",
                                                bin
                                            );
                                            queue.lock().unwrap().extend(bin);
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
            Ok(self.receive_queue.clone())
        })
        // tokio::spawn(async move {
        //     if let Err(e) = WebSocketServerNative::connect_async(
        //         &address,
        //         receive_queue.clone(),
        //     )
        //     .await
        //     {
        //         error!("Server error: {}", e);
        //     }
        // });
    }

    fn get_address(&self) -> Url {
        self.address.clone()
    }

    fn send_block<'a>(
        &'a mut self,
        message: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move { true })
    }
}

impl WebSocketServerInterface<WebSocketServerNative> {
    pub async fn start(
        port: u16,
    ) -> Result<
        WebSocketServerInterface<WebSocketServerNative>,
        WebSocketServerError,
    > {
        let address = format!("127.0.0.1:{}", port);
        let mut websocket = WebSocketServerNative::new(&address.to_string())?;
        websocket.connect().await?;
        Ok(WebSocketServerInterface::new_with_web_socket_server(
            Rc::new(RefCell::new(websocket)),
        ))
    }
}
