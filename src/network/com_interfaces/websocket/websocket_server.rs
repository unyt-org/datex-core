use std::collections::HashMap; // FIXME no-std
use std::sync::Mutex; // FIXME no-std

use crate::stdlib::{
    cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc, time::Duration,
};

use log::debug;
use url::Url;

use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceUUID,
};
use crate::network::com_interfaces::{
    com_interface::ComInterface, com_interface_properties::InterfaceProperties,
    com_interface_socket::ComInterfaceSocket,
};
use crate::utils::uuid::UUID;

pub struct WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    uuid: ComInterfaceUUID,
    pub web_socket_server: Rc<RefCell<WS>>,
    pub web_sockets: HashMap<ComInterfaceSocket, WS>,
    sockets: Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>,
}

#[derive(Debug)]
pub enum WebSocketServerError {
    WebSocketError,
    InvalidPort,
}

pub trait WebSocket {
    fn send_data(&self, message: &[u8]) -> bool;
    fn get_address(&self) -> Url;
    fn connect(
        &mut self,
    ) -> Result<Arc<Mutex<VecDeque<u8>>>, WebSocketServerError>;
}

impl<WS> WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    pub fn new_with_web_socket_server(
        web_socket_server: Rc<RefCell<WS>>,
    ) -> WebSocketServerInterface<WS> {
        WebSocketServerInterface {
            uuid: ComInterfaceUUID(UUID::new()),
            web_sockets: HashMap::new(),
            web_socket_server,
            sockets: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl<WS> ComInterface for WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    fn connect(&mut self) -> Result<(), ComInterfaceError> {
        debug!("Connecting to WebSocket");
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

    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) {
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
