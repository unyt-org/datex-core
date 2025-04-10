use std::{future::Future, pin::Pin, sync::Mutex}; // FIXME no-std

use crate::{
    network::com_interfaces::websocket::websocket_common::WebSocketError,
    stdlib::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc},
};

use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::net::TcpStream;
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::websocket::{
    websocket_client::{WebSocket, WebSocketClientInterface},
    websocket_common::parse_url,
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
pub struct WebSocketClientNative {
    client:
        Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    address: Url,
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
}

impl WebSocketClientNative {
    fn new(address: &str) -> Result<WebSocketClientNative, WebSocketError> {
        let address =
            parse_url(address).map_err(|_| WebSocketError::InvalidURL)?;
        Ok(WebSocketClientNative {
            client: None,
            receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            address,
        })
    }

    async fn connect_async(
        address: &Url,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, WebSocketError>
    {
        info!(
            "Connecting to WebSocket server at {}",
            address.host_str().unwrap()
        );
        let (ws_stream, _) = tokio_tungstenite::connect_async(address)
            .await
            .map_err(|_| WebSocketError::ConnectionError)?;
        Ok(ws_stream)
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
            let stream = WebSocketClientNative::connect_async(&address)
                .await
                .unwrap();
            let (write, read) = stream.split();
            // self.client = Some(write);
            let receive_queue_clone = receive_queue.clone();
            read.for_each(|message| {
                let receive_queue = receive_queue_clone.clone();
                async move {
                    match message {
                        Ok(msg) => {
                            if let Message::Binary(data) = msg {
                                let mut queue = receive_queue.lock().unwrap();
                                queue.extend(data);
                            }
                        }
                        Err(e) => {
                            error!("Error receiving message: {:?}", e)
                        }
                    }
                }
            })
            .await;
            Ok(self.receive_queue.clone())
        })
        // let self_clone: Arc<Mutex<_>> = Arc::new(Mutex::new(self.clone()));
    }

    // async fn send_data(&mut self, message: &[u8]) -> bool {
    //     if let Some(client) = self.client.as_mut() {
    //         debug!("Sending message: {:?}", message);
    //         client.send(Message::Binary(message.to_vec())).await.is_ok()
    //     } else {
    //         false
    //     }
    // }

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

    fn send_data<'a>(
        &'a mut self,
        message: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let client = self.client.as_mut();
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
    pub fn new(
        address: &str,
    ) -> Result<WebSocketClientInterface<WebSocketClientNative>, WebSocketError>
    {
        let websocket = WebSocketClientNative::new(address)?;

        Ok(WebSocketClientInterface::new_with_web_socket(Rc::new(
            RefCell::new(websocket),
        )))
    }
}
