use std::sync::Mutex; // FIXME no-std

use crate::stdlib::{
    cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc, time::Duration,
};

use log::{debug, info};
use url::Url;

use super::websocket_common::WebSocketError;
use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::InterfaceDirection;
use crate::utils::uuid::UUID;
use crate::{
    network::com_interfaces::{
        com_interface::ComInterface,
        com_interface_properties::InterfaceProperties,
        com_interface_socket::ComInterfaceSocket,
    },
    runtime::Context,
};

pub struct WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    pub uuid: ComInterfaceUUID,
    pub web_socket: Rc<RefCell<WS>>,
    context: Rc<RefCell<Context>>,
    socket: Option<Rc<RefCell<ComInterfaceSocket>>>,
}

pub trait WebSocket {
    fn send_data(&mut self, message: &[u8]) -> bool;
    fn get_address(&self) -> Url;
    fn connect(&mut self) -> Result<Arc<Mutex<VecDeque<u8>>>, WebSocketError>;
}

impl<WS> WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    pub fn new_with_web_socket(
        context: Rc<RefCell<Context>>,
        web_socket: Rc<RefCell<WS>>,
    ) -> WebSocketClientInterface<WS> {
        WebSocketClientInterface {
            uuid: ComInterfaceUUID(UUID::new()),
            web_socket,
            context,
            socket: None,
        }
    }
}

impl<WS> ComInterface for WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) {
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

    fn connect(&mut self) -> Result<(), ComInterfaceError> {
        debug!("Connecting to WebSocket");
        let receive_queue = self
            .web_socket
            .borrow_mut()
            .connect()
            .map_err(|_| ComInterfaceError::ConnectionError)?;
        let socket = self.create_socket_default(
            receive_queue,
        );
        self.socket = Some(Rc::new(RefCell::new(socket)));
        info!("Adding WebSocket");

        Ok(())
    }

    fn get_uuid(&self) -> ComInterfaceUUID {
        self.uuid.clone()
    }
}
