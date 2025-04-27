use log::error;

use crate::datex_values::Endpoint;
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::socket_provider::MultipleSocketProvider;
use crate::{delegate_com_interface_info, set_sync_opener};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::super::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::ComInterfaceState;

pub type OnSendCallback = dyn Fn(&[u8], ComInterfaceSocketUUID) -> Pin<Box<dyn Future<Output = bool>>>
    + 'static;

pub struct BaseInterface {
    name: String,
    info: ComInterfaceInfo,
    on_send: Option<Box<OnSendCallback>>,
}
impl Default for BaseInterface {
    fn default() -> Self {
        Self::new("unknown")
    }
}

use datex_macros::{com_interface, create_opener};
use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum BaseInterfaceError {
    SendError,
    ReceiveError,
    SocketNotFound,
}

#[com_interface]
impl BaseInterface {
    pub fn new_with_single_socket(
        name: &str,
        direction: InterfaceDirection,
    ) -> BaseInterface {
        let interface = BaseInterface::new(name);
        let socket =
            ComInterfaceSocket::new(interface.get_uuid().clone(), direction, 1);
        let socket_uuid = socket.uuid.clone();
        let socket = Arc::new(Mutex::new(socket));
        interface.add_socket(socket);
        interface
            .register_socket_endpoint(socket_uuid, Endpoint::default(), 1)
            .unwrap();
        interface
    }

    pub fn new(name: &str) -> BaseInterface {
        BaseInterface {
            name: name.to_string(),
            info: ComInterfaceInfo::default(),
            on_send: None,
        }
    }

    #[create_opener]
    fn open(&mut self) -> Result<(), ()> {
        Ok(())
    }

    pub fn register_new_socket(
        &mut self,
        direction: InterfaceDirection,
    ) -> ComInterfaceSocketUUID {
        let socket =
            ComInterfaceSocket::new(self.get_uuid().clone(), direction, 1);
        let socket_uuid = socket.uuid.clone();
        let socket = Arc::new(Mutex::new(socket));
        self.add_socket(socket);
        socket_uuid
    }
    pub fn register_new_socket_with_endpoint(
        &mut self,
        direction: InterfaceDirection,
        endpoint: Endpoint,
    ) -> ComInterfaceSocketUUID {
        let socket_uuid = self.register_new_socket(direction);
        self.register_socket_endpoint(socket_uuid.clone(), endpoint, 1)
            .unwrap();
        socket_uuid
    }

    pub fn set_on_send_callback(
        &mut self,
        on_send: Box<OnSendCallback>,
    ) -> &mut Self {
        self.on_send = Some(on_send);
        self
    }

    pub fn receive(
        &mut self,
        receiver_socket_uuid: ComInterfaceSocketUUID,
        data: Vec<u8>,
    ) -> Result<(), BaseInterfaceError> {
        if let Some(socket) = self.get_socket_with_uuid(receiver_socket_uuid) {
            let socket = socket.lock().unwrap();
            let receive_queue = socket.get_receive_queue();
            receive_queue.lock().unwrap().extend(data);
            Ok(())
        } else {
            error!("Socket not found");
            Err(BaseInterfaceError::SocketNotFound)
        }
    }
}

impl MultipleSocketProvider for BaseInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

impl ComInterface for BaseInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        if !self.has_socket_with_uuid(socket_uuid.clone()) {
            return Box::pin(async move { false });
        }
        if let Some(on_send) = &self.on_send {
            on_send(block, socket_uuid)
        } else {
            Box::pin(async move { false })
        }
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            interface_type: self.name.clone(),
            channel: self.name.clone(),
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            ..InterfaceProperties::default()
        }
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }
    delegate_com_interface_info!();
    set_sync_opener!(open);
}
