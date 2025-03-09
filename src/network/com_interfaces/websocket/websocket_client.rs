use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, Result};
use url::Url;

use crate::{
    network::com_interfaces::{
        com_interface::ComInterface,
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::ComInterfaceSocket,
    },
    runtime::Context,
    utils::logger::{self, Logger},
};
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;

pub struct WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    pub uuid: ComInterfaceUUID,
    pub web_socket: Rc<RefCell<WS>>,
    pub logger: Option<Logger>,
    context: Rc<RefCell<Context>>,
    socket: Option<Rc<RefCell<ComInterfaceSocket>>>,
}

pub trait WebSocket {
    fn send_data(&self, message: &[u8]) -> bool;
    fn get_address(&self) -> Url;
    fn connect(&mut self) -> Result<Arc<Mutex<VecDeque<u8>>>>;
}

impl<WS> WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    pub fn new_with_web_socket(
        context: Rc<RefCell<Context>>,
        web_socket: Rc<RefCell<WS>>,
        logger: Option<Logger>,
    ) -> WebSocketClientInterface<WS> {
        return WebSocketClientInterface {
            uuid: ComInterfaceUUID::new(),
            web_socket,
            context,
            logger,
            socket: None,
        };
    }
}

impl<WS> ComInterface for WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> () {
        // TODO: what happens if socket != self.socket? (only one socket exists)
        self.web_socket.borrow_mut().send_data(block);
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "websocket".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    fn get_sockets(&self) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>> {
        match self.socket.clone() {
            Some(socket) => Rc::new(RefCell::new(vec![socket.clone()])),
            None => Rc::new(RefCell::new(vec![])),
        }
    }

    fn connect(&mut self) -> Result<()> {
        if let Some(logger) = &self.logger {
            logger.debug(&"Connecting to WebSocket");
        }
        let receive_queue = self.web_socket.borrow_mut().connect()?;
        let socket = ComInterfaceSocket::new_with_receive_queue(
            self.context.clone(),
            receive_queue,
            self.logger.clone(),
        );
        self.socket = Some(Rc::new(RefCell::new(socket)));
        if let Some(logger) = &self.logger {
            logger.success(&"Adding WebSocket");
        }

        Ok(())
    }

    fn get_uuid(&self) -> ComInterfaceUUID {
        self.uuid.clone()
    }
}
