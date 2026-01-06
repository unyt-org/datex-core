use core::prelude::rust_2024::*;
use core::result::Result;
use log::error;

use super::super::com_interface::ComInterface;
use crate::network::com_hub::errors::ComHubError;
use crate::network::com_interfaces::com_interface::ComInterfaceState;
use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceFactory, ComInterfaceInfo,
    ComInterfaceSockets,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::socket_provider::MultipleSocketProvider;
use crate::std_sync::Mutex;
use crate::stdlib::boxed::Box;
use crate::stdlib::pin::Pin;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::sync::Arc;
use crate::stdlib::vec::Vec;
use crate::values::core_values::endpoint::Endpoint;
use crate::{delegate_com_interface_info, set_sync_opener};
use core::future::Future;
use core::time::Duration;
use serde::{Deserialize, Serialize};

pub type OnSendCallback = dyn Fn(&[u8], ComInterfaceSocketUUID) -> Pin<Box<dyn Future<Output = bool>>>
    + 'static;

pub struct BaseInterface {
    info: ComInterfaceInfo,
    on_send: Option<Box<OnSendCallback>>,
    properties: InterfaceProperties,
}
impl Default for BaseInterface {
    fn default() -> Self {
        Self::new_with_name("unknown")
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
    InterfaceNotFound,
    InvalidInput(String),
    ComHubError(ComHubError),
}

impl From<ComHubError> for BaseInterfaceError {
    fn from(err: ComHubError) -> Self {
        BaseInterfaceError::ComHubError(err)
    }
}

#[com_interface]
impl BaseInterface {
    pub fn new_with_single_socket(
        name: &str,
        direction: InterfaceDirection,
    ) -> BaseInterface {
        let interface = BaseInterface::new_with_name(name);
        let socket =
            ComInterfaceSocket::init(interface.uuid().clone(), direction, 1);
        let socket_uuid = socket.uuid.clone();
        let socket = Arc::new(Mutex::new(socket));
        interface.add_socket(socket);
        interface
            .register_socket_endpoint(socket_uuid, Endpoint::default(), 1)
            .unwrap();
        interface
    }

    pub fn new() -> BaseInterface {
        Self::new_with_name("unknown")
    }

    pub fn new_with_name(name: &str) -> BaseInterface {
        Self::new_with_properties(InterfaceProperties {
            interface_type: name.to_string(),
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            ..InterfaceProperties::default()
        })
    }
    pub fn new_with_properties(
        properties: InterfaceProperties,
    ) -> BaseInterface {
        BaseInterface {
            info: ComInterfaceInfo::default(),
            properties,
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
            ComInterfaceSocket::init(self.uuid().clone(), direction, 1);
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
        match self.get_socket_with_uuid(receiver_socket_uuid) {
            Some(socket) => {
                socket.try_lock().unwrap().queue_outgoing_block(&data);
                Ok(())
            }
            _ => {
                error!("Socket not found");
                Err(BaseInterfaceError::SocketNotFound)
            }
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
        self.properties.clone()
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }
    delegate_com_interface_info!();
    set_sync_opener!(open);
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct BaseInterfaceSetupData(pub InterfaceProperties);

impl ComInterfaceFactory<BaseInterfaceSetupData> for BaseInterface {
    fn create(
        setup_data: BaseInterfaceSetupData,
    ) -> Result<BaseInterface, ComInterfaceError> {
        Ok(BaseInterface::new_with_properties(setup_data.0))
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties::default()
    }
}
