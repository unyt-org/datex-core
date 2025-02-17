use std::{cell::RefCell, collections::VecDeque, rc::Rc, sync::{Arc, Mutex}};

use anyhow::{anyhow, Result};
use url::Url;

use crate::network::com_interfaces::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::ComInterfaceSocket,
};

use super::com_interface::ComInterface;

pub struct WebSocketClientInterface<WS> where WS: WebSocket {
    pub websocket: WS,
    socket: Option<Rc<RefCell<ComInterfaceSocket>>>,
}

pub trait WebSocket {
    fn send_data(&self, message: &[u8]) -> bool;
    fn get_address(&self) -> Url;
    fn connect(&self) -> Result<Arc<Mutex<VecDeque<u8>>>>;
}

pub fn parse_url(address: &str) -> Result<Url> {
    let address = if address.contains("://") {
        address.to_string()
    } else {
        format!("wss://{}", address)
    };

    let mut url =
        Url::parse(&address).map_err(|_| anyhow!("Invalid URL"))?;
        match url.scheme() {
            "https" => url.set_scheme("wss").unwrap(),
            "http" => url.set_scheme("ws").unwrap(),
            "wss" | "ws" => (),
            _ => return Err(anyhow!("Invalid URL scheme")),
        }
    Ok(url)
}

impl<WS> WebSocketClientInterface<WS> where WS: WebSocket {
    pub fn new_with_web_socket(web_socket: WS) -> WebSocketClientInterface<WS> {
        return WebSocketClientInterface {
            websocket: web_socket,
            socket: None,
        };
    }

    fn set_socket(&mut self, socket: Rc<RefCell<ComInterfaceSocket>>) {
        self.socket = Some(socket);
    }
}

impl<WS> ComInterface for WebSocketClientInterface<WS> where WS: WebSocket {

    fn connect(&mut self) -> Result<()> {
        let receive_queue = self.websocket.connect()?;
        let socket = ComInterfaceSocket {
            receive_queue,
            ..Default::default()
        };
        self.socket = Some(Rc::new(RefCell::new(socket)));
        Ok(())
    }

    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> () {
        // TODO: what happens if socket != self.socket? (only one socket exists)
        self.websocket.send_data(block);
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "websocket".to_string(),
            name: None,
            direction: InterfaceDirection::IN_OUT,
            reconnect_interval: None,
            latency: 0,
            bandwidth: 1000,
            continuous_connection: true,
            allow_redirects: true,
        }
    }

    fn get_sockets(
        &self,
    ) -> std::rc::Rc<std::cell::RefCell<Vec<std::rc::Rc<std::cell::RefCell<ComInterfaceSocket>>>>>
    {
        match self.socket.clone() {
            Some(socket) => Rc::new(std::cell::RefCell::new(vec![socket.clone()])),
            None => Rc::new(std::cell::RefCell::new(vec![])),
        }
    }
}
