use core::prelude::rust_2024::*;
use core::result::Result;
use std::collections::HashMap;

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
    sender: HashMap<ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>>,
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
    fn create_and_init_socket(
        &mut self,
        direction: InterfaceDirection,
    ) -> ComInterfaceSocketUUID {
        let (uuid, sender) = self
            .com_interface
            .borrow()
            .socket_manager()
            .lock()
            .unwrap()
            .create_and_init_socket(direction, 1);
        self.sender.insert(uuid.clone(), sender);
        uuid
    }
    pub fn register_new_socket_with_endpoint(
        &mut self,
        direction: InterfaceDirection,
        endpoint: Endpoint,
    ) -> ComInterfaceSocketUUID {
        let socket_uuid = self.create_and_init_socket(direction);

        self.com_interface
            .borrow()
            .socket_manager()
            .lock()
            .unwrap()
            .register_socket_with_endpoint(socket_uuid.clone(), endpoint, 1)
            .unwrap();
        socket_uuid
    }

    pub fn receive(
        &mut self,
        receiver_socket_uuid: ComInterfaceSocketUUID,
        data: Vec<u8>,
    ) -> Result<(), BaseInterfaceError> {
        if let Some(sender) = self.sender.get_mut(&receiver_socket_uuid) {
            sender
                .start_send(data)
                .map_err(|_| BaseInterfaceError::ReceiveError)?;
            Ok(())
        } else {
            Err(BaseInterfaceError::SocketNotFound)
        }
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

#[derive(Serialize, Deserialize, Default)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct BaseInterfaceSetupData(pub InterfaceProperties);

impl BaseInterfaceSetupData {
    pub fn new(properties: InterfaceProperties) -> Self {
        BaseInterfaceSetupData(properties)
    }
}

impl ComInterfaceFactory for BaseInterface {
    type SetupData = BaseInterfaceSetupData;

    fn create(
        setup_data: BaseInterfaceSetupData,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<BaseInterface, ComInterfaceError> {
        Ok(BaseInterface {
            sender: HashMap::new(),
            properties: setup_data.0,
            on_send: None,
            com_interface,
        })
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties::default()
    }
}
