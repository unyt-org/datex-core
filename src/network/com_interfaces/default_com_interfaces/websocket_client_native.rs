use std::sync::Mutex; // FIXME no-std

use crate::{
    network::com_interfaces::websocket::websocket_common::WebSocketError,
    stdlib::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc},
};

use futures_util::StreamExt;
use log::{debug, info};
use tokio::net::TcpStream;
use url::Url;
use websocket::{stream::sync::NetworkStream, sync::Client, ClientBuilder};

use crate::network::com_interfaces::websocket::{
    websocket_client::{WebSocket, WebSocketClientInterface},
    websocket_common::parse_url,
};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
pub struct WebSocketClientNative {
    client: Option<Client<Box<dyn NetworkStream + Send>>>,
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
    fn connect(&mut self) -> Result<Arc<Mutex<VecDeque<u8>>>, WebSocketError> {
        let mut client = ClientBuilder::new(self.address.as_str())
            .map_err(|_| WebSocketError::InvalidURL)?;
        info!("a");
        if self.address.scheme() == "wss" {
            // TODO SSL
            self.client = Some(client.connect(None).unwrap());
        } else {
            self.client = Some(client.connect(None).unwrap());
        }
        info!("b");

        Ok(self.receive_queue.clone())
    }

    fn send_data(&mut self, message: &[u8]) -> bool {
        info!("c");

        if let Some(client) = self.client.as_mut() {
            let owned_message =
                websocket::OwnedMessage::Binary(message.to_vec());
            debug!("Sending message: {:?}", owned_message);
            client.send_message(&owned_message).is_ok()
        } else {
            false
        }
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
