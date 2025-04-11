use std::{future::Future, pin::Pin, sync::Mutex}; // FIXME no-std

use crate::{
    network::com_interfaces::websocket::websocket_common::WebSocketError,
    stdlib::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc},
};

use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::{net::TcpStream, spawn};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::websocket::{
    websocket_client::{WebSocket, WebSocketClientInterface},
    websocket_common::parse_url,
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
pub struct WebSocketClientNative {
    tx_stream:
        Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    address: Url,
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
}

impl WebSocketClientNative {
    fn new(address: &str) -> Result<WebSocketClientNative, WebSocketError> {
        let address =
            parse_url(address).map_err(|_| WebSocketError::InvalidURL)?;
        Ok(WebSocketClientNative {
            tx_stream: None,
            receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            address,
        })
    }
}

impl WebSocket for WebSocketClientNative {
    fn connect<'a>(
        &'a mut self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<Arc<Mutex<VecDeque<u8>>>, WebSocketError>,
                > + 'a,
        >,
    > {
        let address = self.address.clone();
        let receive_queue = self.receive_queue.clone();

        Box::pin(async move {
            info!(
                "Connecting to WebSocket server at {}",
                address.host_str().unwrap()
            );
            let (stream, _) = tokio_tungstenite::connect_async(address)
                .await
                .map_err(|_| WebSocketError::ConnectionError)?;
            let (write, mut read) = stream.split();
            let receive_queue_clone = receive_queue.clone();
            self.tx_stream = Some(write);
            spawn(async move {
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Binary(data)) => {
                            let mut queue = receive_queue_clone.lock().unwrap();
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
            Ok(self.receive_queue.clone())
        })
    }

    fn get_address(&self) -> Url {
        self.address.clone()
    }

    fn get_com_interface_sockets(
        &self,
    ) -> Rc<
        RefCell<
            crate::network::com_interfaces::com_interface::ComInterfaceSockets,
        >,
    > {
        todo!()
    }

    fn send_block<'a>(
        &'a mut self,
        message: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let client = self.tx_stream.as_mut();
            if client.is_none() {
                error!("Client is not connected");
                return false;
            }
            debug!("Sending message: {:?}", message);

            let client = client.unwrap();
            client
                .send(Message::Binary(message.to_vec()))
                .await
                .map_err(|e| {
                    error!("Error sending message: {:?}", e);
                    false
                })
                .is_ok()
        })
    }
}

impl WebSocketClientInterface<WebSocketClientNative> {
    pub async fn start(
        address: &str,
    ) -> Result<WebSocketClientInterface<WebSocketClientNative>, WebSocketError>
    {
        let mut websocket = WebSocketClientNative::new(address)?;
        websocket.connect().await?;

        Ok(WebSocketClientInterface::new_with_web_socket(Rc::new(
            RefCell::new(websocket),
        )))
    }
}
