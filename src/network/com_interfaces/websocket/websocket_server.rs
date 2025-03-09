use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
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

pub struct WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    uuid: ComInterfaceUUID,
    pub web_socket_server: Rc<RefCell<WS>>,
    pub web_sockets: HashMap<ComInterfaceSocket, WS>,
    pub logger: Option<Logger>,
    sockets: Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>,
}

pub trait WebSocket {
    fn send_data(&self, message: &[u8]) -> bool;
    fn get_address(&self) -> Url;
    fn connect(&mut self) -> Result<Arc<Mutex<VecDeque<u8>>>>;
}

impl<WS> WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    pub fn new_with_web_socket_server(
        context: Rc<RefCell<Context>>,
        web_socket_server: Rc<RefCell<WS>>,
        logger: Option<Logger>,
    ) -> WebSocketServerInterface<WS> {
        return WebSocketServerInterface {
            uuid: ComInterfaceUUID::new(),
            web_sockets: HashMap::new(),
            web_socket_server,
            logger,
            sockets: Rc::new(RefCell::new(Vec::new())),
        };
    }
}

impl<WS> ComInterface for WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    fn connect(&mut self) -> Result<()> {
        if let Some(logger) = &self.logger {
            logger.debug(&"Connecting to WebSocket");
        }
        //   let receive_queue = self.websocket.borrow_mut().connect()?;
        //   let socket = ComInterfaceSocket::new_with_logger_and_receive_queue(
        // 	self.logger.clone(),
        // 	receive_queue,
        //   );
        //   self.sockets = Some(Rc::new(RefCell::new(socket)));
        //   if let Some(logger) = &self.logger {
        // 	logger.success(&"Adding WebSocket");
        //   }

        Ok(())
    }

    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> () {
        // TODO: what happens if socket != self.socket? (only one socket exists)
        //   self.websocket.borrow_mut().send_data(block);
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
        self.sockets.clone()
    }

    fn get_uuid(&self) -> ComInterfaceUUID {
        self.uuid.clone()
    }
}
