use std::{
    cell::{Ref, RefCell},
    collections::VecDeque,
    net::TcpStream,
    rc::Rc,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use url::Url;
use websocket::{
    sync::{stream::TlsStream, Client},
    ClientBuilder,
};

use crate::{
    crypto::{self, crypto::Crypto},
    network::com_interfaces::websocket::{
        websocket_client::{WebSocket, WebSocketClientInterface},
        websocket_common::parse_url,
    },
    runtime::Context,
};

struct WebSocketNative {
    client: Client<TlsStream<TcpStream>>,
    address: Url,
}

impl WebSocketNative {
    fn new(address: &str) -> Result<WebSocketNative> {
        let address = parse_url(address)?;

        let mut client = ClientBuilder::new(address.as_str())
            .unwrap()
            .connect_secure(None)
            .unwrap();

        for message in client.incoming_messages() {
            println!("Recv: {:?}", message.unwrap());
        }

        Ok(WebSocketNative { client, address })
    }
}

impl WebSocket for WebSocketNative {
    fn connect(&mut self) -> Result<Arc<Mutex<VecDeque<u8>>>> {
        todo!()
    }

    fn send_data(&self, message: &[u8]) -> bool {
        todo!()
    }

    fn get_address(&self) -> Url {
        self.address.clone()
    }
}

impl WebSocketClientInterface<WebSocketNative> {
    pub fn new(
        crypto: Rc<RefCell<Context>>,
        address: &str,
    ) -> Result<WebSocketClientInterface<WebSocketNative>> {
        let websocket = WebSocketNative::new(address)?;

        Ok(WebSocketClientInterface::new_with_web_socket(
            crypto,
            Rc::new(RefCell::new(websocket)),
            None,
        ))
    }
}
