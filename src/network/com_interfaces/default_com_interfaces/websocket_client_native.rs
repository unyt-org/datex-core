use std::sync::Mutex; // FIXME no-std

use crate::{
    network::com_interfaces::websocket::websocket_common::WebSocketError,
    stdlib::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc},
};

use url::Url;
use websocket::{stream::sync::NetworkStream, sync::Client, ClientBuilder};

use crate::{
    network::com_interfaces::websocket::{
        websocket_client::{WebSocket, WebSocketClientInterface},
        websocket_common::parse_url,
    },
    runtime::Context,
};

pub struct WebSocketNative {
    client: Option<Client<Box<dyn NetworkStream + Send>>>,
    address: Url,
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
}

impl WebSocketNative {
    fn new(address: &str) -> Result<WebSocketNative, WebSocketError> {
        let address =
            parse_url(address).map_err(|_| WebSocketError::InvalidURL)?;
        Ok(WebSocketNative {
            client: None,
            receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            address,
        })
    }
}

impl WebSocket for WebSocketNative {
    fn connect(&mut self) -> Result<Arc<Mutex<VecDeque<u8>>>, WebSocketError> {
        let mut client = ClientBuilder::new(self.address.as_str())
            .map_err(|_| WebSocketError::InvalidURL)?;
        if self.address.scheme() == "wss" {
            // TODO SSL
            self.client = Some(client.connect(None).unwrap());
        } else {
            self.client = Some(client.connect(None).unwrap());
        }
        Ok(self.receive_queue.clone())
    }

    fn send_data(&mut self, message: &[u8]) -> bool {
        if let Some(client) = self.client.as_mut() {
            let owned_message =
                websocket::OwnedMessage::Binary(message.to_vec());
            client.send_message(&owned_message).is_ok()
        } else {
            false
        }
    }

    fn get_address(&self) -> Url {
        self.address.clone()
    }
}

impl WebSocketClientInterface<WebSocketNative> {
    pub fn new(
        ctx: Rc<RefCell<Context>>,
        address: &str,
    ) -> Result<WebSocketClientInterface<WebSocketNative>, WebSocketError> {
        let websocket = WebSocketNative::new(address)?;

        Ok(WebSocketClientInterface::new_with_web_socket(
            ctx,
            Rc::new(RefCell::new(websocket)),
        ))
    }
}
