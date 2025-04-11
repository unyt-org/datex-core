use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
// FIXME no-std
use std::sync::Mutex; // FIXME no-std

use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::stdlib::{
    cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc, time::Duration,
};

use log::debug;
use strum::Display;
use url::Url;

use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceSockets, ComInterfaceUUID,
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
    // sockets: Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>,
    com_interface_sockets: Rc<RefCell<ComInterfaceSockets>>,
}

#[derive(Debug, Display)]
pub enum WebSocketServerError {
    WebSocketError,
    InvalidPort,
}
impl std::error::Error for WebSocketServerError {}

pub trait WebSocket {
    fn send_block<'a>(
        &'a mut self,
        message: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
    fn connect<'a>(
        &'a mut self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Arc<Mutex<VecDeque<u8>>>,
                        WebSocketServerError,
                    >,
                > + 'a,
        >,
    >;
    fn get_address(&self) -> Url;
}

impl<WS> WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    pub(crate) fn new_with_web_socket_server(
        web_socket_server: Rc<RefCell<WS>>,
    ) -> WebSocketServerInterface<WS> {
        WebSocketServerInterface {
            uuid: ComInterfaceUUID(UUID::new()),
            web_sockets: HashMap::new(),
            web_socket_server,
            com_interface_sockets: Rc::new(RefCell::new(
                ComInterfaceSockets::default(),
            )),
        }
    }
}

impl<WS> ComInterface for WebSocketServerInterface<WS>
where
    WS: WebSocket,
{
    fn open<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), ComInterfaceError>> + 'a>> {
        debug!("Spinning up websocket server");
        Box::pin(async move {
            let receive_queue = self
                .web_socket_server
                .borrow_mut()
                .connect()
                .await
                .map_err(|_| ComInterfaceError::ConnectionError)?;
            //   let socket = ComInterfaceSocket::new_with_logger_and_receive_queue(
            // 	self.logger.clone(),
            // 	receive_queue,
            //   );
            //   self.sockets = Some(Rc::new(RefCell::new(socket)));
            //   if let Some(logger) = &self.logger {
            // 	logger.success(&"Adding WebSocket");
            //   }

            Ok(())
        })
    }

    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: Option<ComInterfaceSocketUUID>,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // self.we
        // self.websocket.borrow_mut().send_data(block);
        // let web_socket  =
        Box::pin(async move {
            // TODO
            self.web_sockets
                .values()
                .next()
                .unwrap()
                .send_block(block)
                .await
        })
    }
    // fn send_block(
    //     &mut self,
    //     block: &[u8],
    //     socket: Option<&ComInterfaceSocket>,
    // ) {
    //     // TODO: what happens if socket != self.socket? (only one socket exists)
    //     //   self.websocket.borrow_mut().send_data(block);
    // }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "websocketserver".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    fn get_uuid<'a>(&'a self) -> &'a ComInterfaceUUID {
        &self.uuid
    }

    fn get_sockets(
        &self,
    ) -> Rc<
        RefCell<
            crate::network::com_interfaces::com_interface::ComInterfaceSockets,
        >,
    > {
        self.com_interface_sockets.clone()
    }
}
