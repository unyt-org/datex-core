use std::{
  cell::RefCell,
  collections::VecDeque,
  rc::Rc,
  sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use url::Url;

use crate::{
  network::com_interfaces::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::ComInterfaceSocket,
  },
  utils::logger::{self, Logger},
};

use super::com_interface::ComInterface;

pub struct WebSocketClientInterface<WS>
where
  WS: WebSocket,
{
  pub websocket: Rc<RefCell<WS>>,
  pub logger: Option<Logger>,
  socket: Option<Rc<RefCell<ComInterfaceSocket>>>,
}

pub trait WebSocket {
  fn send_data(&self, message: &[u8]) -> bool;
  fn get_address(&self) -> Url;
  fn connect(&mut self) -> Result<Arc<Mutex<VecDeque<u8>>>>;
}

pub fn parse_url(address: &str) -> Result<Url> {
  let address = if address.contains("://") {
    address.to_string()
  } else {
    format!("wss://{}", address)
  };

  let mut url = Url::parse(&address).map_err(|_| anyhow!("Invalid URL"))?;
  match url.scheme() {
    "https" => url.set_scheme("wss").unwrap(),
    "http" => url.set_scheme("ws").unwrap(),
    "wss" | "ws" => (),
    _ => return Err(anyhow!("Invalid URL scheme")),
  }
  Ok(url)
}

impl<WS> WebSocketClientInterface<WS>
where
  WS: WebSocket,
{
  pub fn new_with_web_socket(
    web_socket: Rc<RefCell<WS>>,
    logger: Option<Logger>,
  ) -> WebSocketClientInterface<WS> {
    return WebSocketClientInterface {
      websocket: web_socket,
      logger,
      socket: None,
    };
  }
}

impl<WS> ComInterface for WebSocketClientInterface<WS>
where
  WS: WebSocket,
{
  fn connect(&mut self) -> Result<()> {
    if let Some(logger) = &self.logger {
      logger.debug(&"Connecting to WebSocket");
    }
    let receive_queue = self.websocket.borrow_mut().connect()?;
    let socket = ComInterfaceSocket::new_with_logger_and_receive_queue(
      self.logger.clone(),
      receive_queue,
    );
    self.socket = Some(Rc::new(RefCell::new(socket)));
    if let Some(logger) = &self.logger {
      logger.success(&"Adding WebSocket");
    }

    Ok(())
  }

  fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> () {
    // TODO: what happens if socket != self.socket? (only one socket exists)
    self.websocket.borrow_mut().send_data(block);
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

  fn get_sockets(&self) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>> {
    match self.socket.clone() {
      Some(socket) => Rc::new(RefCell::new(vec![socket.clone()])),
      None => Rc::new(RefCell::new(vec![])),
    }
  }
}
