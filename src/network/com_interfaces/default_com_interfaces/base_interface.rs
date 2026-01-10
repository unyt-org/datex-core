use core::prelude::rust_2024::*;
use core::result::Result;

use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::{
    ComInterfaceFactory, ComInterfaceImplementation,
};
use crate::network::{
    com_hub::errors::ComHubError,
    com_interfaces::com_interface::properties::InterfaceDirection,
};

use crate::network::com_interfaces::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface::socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::stdlib::boxed::Box;
use crate::stdlib::cell::RefCell;
use crate::stdlib::pin::Pin;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::values::core_values::endpoint::Endpoint;
use core::future::Future;
use serde::{Deserialize, Serialize};

pub type OnSendCallback = dyn Fn(&[u8], ComInterfaceSocketUUID) -> Pin<Box<dyn Future<Output = bool>>>
    + 'static;

pub struct BaseInterface {
    on_send: Option<Box<OnSendCallback>>,
    properties: InterfaceProperties,
    com_interface: Rc<RefCell<ComInterface>>,
}

use crate::task::UnboundedSender;
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

impl BaseInterface {
    // TODO
    // pub fn new_with_single_socket(
    //     name: &str,
    //     direction: InterfaceDirection,
    // ) -> BaseInterface {
    //     let interface = BaseInterface::new_with_name(name);
    //     let socket =
    //         ComInterfaceSocket::init(interface.uuid().clone(), direction, 1);
    //     let socket_uuid = socket.uuid.clone();
    //     let socket = Arc::new(Mutex::new(socket));
    //     interface.add_socket(socket);
    //     interface
    //         .register_socket_endpoint(socket_uuid, Endpoint::default(), 1)
    //         .unwrap();
    //     interface
    // }
    //
    // pub fn new() -> BaseInterface {
    //     Self::new_with_name("unknown")
    // }
    //
    // pub fn new_with_name(name: &str) -> BaseInterface {
    //     Self::new_with_properties(InterfaceProperties {
    //         interface_type: name.to_string(),
    //         round_trip_time: Duration::from_millis(0),
    //         max_bandwidth: u32::MAX,
    //         ..InterfaceProperties::default()
    //     })
    // }

    pub fn create_and_init_socket(
        &mut self,
        direction: InterfaceDirection,
    ) -> (ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>) {
        self.com_interface
            .borrow()
            .socket_manager()
            .lock()
            .unwrap()
            .create_and_init_socket(direction, 1)
    }
    pub fn register_new_socket_with_endpoint(
        &mut self,
        direction: InterfaceDirection,
        endpoint: Endpoint,
    ) -> (ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>) {
        let (socket_uuid, sender) = self.create_and_init_socket(direction);

        self.com_interface
            .borrow()
            .socket_manager()
            .lock()
            .unwrap()
            .register_socket_with_endpoint(socket_uuid.clone(), endpoint, 1)
            .unwrap();

        (socket_uuid, sender)
    }

    pub fn set_on_send_callback(
        &mut self,
        on_send: Box<OnSendCallback>,
    ) -> &mut Self {
        self.on_send = Some(on_send);
        self
    }
}

impl ComInterfaceImplementation for BaseInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        if let Some(on_send) = &self.on_send {
            on_send(block, socket_uuid)
        } else {
            Box::pin(async move { false })
        }
    }

    fn get_properties(&self) -> InterfaceProperties {
        self.properties.clone()
    }

    fn handle_close<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }

    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct BaseInterfaceSetupData(pub InterfaceProperties);

impl ComInterfaceFactory for BaseInterface {
    type SetupData = BaseInterfaceSetupData;

    fn create(
        setup_data: BaseInterfaceSetupData,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<BaseInterface, ComInterfaceError> {
        Ok(BaseInterface {
            properties: setup_data.0,
            on_send: None,
            com_interface,
        })
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties::default()
    }
}
