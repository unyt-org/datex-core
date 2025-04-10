use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex; // FIXME no-std

use crate::stdlib::{
    cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc, time::Duration,
};

use log::{debug, info};
use url::Url;

use super::websocket_common::WebSocketError;
use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::{
    com_interface::ComInterface, com_interface_properties::InterfaceProperties,
    com_interface_socket::ComInterfaceSocket,
};
use crate::utils::uuid::UUID;

pub struct WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    pub uuid: ComInterfaceUUID,
    pub web_socket: Rc<RefCell<WS>>,
}

pub trait WebSocket {
    fn send_data<'a>(
        &'a mut self,
        message: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
    fn get_address(&self) -> Url;
    fn connect<'a>(
        &'a mut self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<Arc<Mutex<VecDeque<u8>>>, WebSocketError>,
                > + 'a,
        >,
    >; //Result<Arc<Mutex<VecDeque<u8>>>, WebSocketError>;
    fn get_com_interface_sockets(&self) -> Rc<RefCell<ComInterfaceSockets>>;
}

impl<WS> WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    pub fn new_with_web_socket(
        web_socket: Rc<RefCell<WS>>,
    ) -> WebSocketClientInterface<WS> {
        WebSocketClientInterface {
            uuid: ComInterfaceUUID(UUID::new()),
            web_socket,
        }
    }
}

impl<WS> ComInterface for WebSocketClientInterface<WS>
where
    WS: WebSocket,
{
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: Option<&ComInterfaceSocket>,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // self.we
        // self.websocket.borrow_mut().send_data(block);
        Box::pin(async move {
            let ws = &mut self.web_socket.borrow_mut();
            ws.send_data(block).await
        })
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "websocket".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    fn get_sockets(&self) -> Rc<RefCell<ComInterfaceSockets>> {
        let sockets = self.web_socket.borrow();
        sockets.get_com_interface_sockets()
    }

    fn open<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), ComInterfaceError>> + 'a>> {
        Box::pin(async move {
            // FIXME add this back when open is async
            debug!("Connecting to WebSocket");
            let receive_queue = self
                .web_socket
                .borrow_mut()
                .connect()
                .await
                .map_err(|_| ComInterfaceError::ConnectionError)?;

            // TODO: get endpoint and call register_endpoint_socket after add_socket
            let socket = self.create_socket_default(receive_queue);
            self.add_socket(Rc::new(RefCell::new(socket)));
            info!("Adding WebSocket");
            Ok(())
        })
    }

    fn get_uuid(&self) -> ComInterfaceUUID {
        self.uuid.clone()
    }
}
