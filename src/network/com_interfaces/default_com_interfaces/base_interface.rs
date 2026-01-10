use core::prelude::rust_2024::*;
use core::result::Result;
use std::collections::HashMap;

use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::{
    ComInterfaceFactory, ComInterfaceImplementation,
};
use crate::network::com_interfaces::com_interface::state::ComInterfaceState;
use crate::network::{
    com_hub::errors::ComHubError,
    com_interfaces::com_interface::properties::InterfaceDirection,
};

use crate::network::com_interfaces::com_interface::properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceInfo,
};
use crate::stdlib::boxed::Box;
use crate::stdlib::cell::RefCell;
use crate::stdlib::pin::Pin;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::values::core_values::endpoint::Endpoint;
use core::future::Future;

pub type OnSendCallback = dyn Fn(&[u8], ComInterfaceSocketUUID) -> Pin<Box<dyn Future<Output = bool>>>
    + 'static;

pub struct BaseInterface {
    on_send: Box<OnSendCallback>,
    properties: InterfaceProperties,
    com_interface: Rc<ComInterface>,
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

pub struct BaseInterfaceHolder {
    sender: HashMap<ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>>,
    pub com_interface: Rc<ComInterface>,
}
impl BaseInterfaceHolder {
    pub fn new(setup_data: BaseInterfaceSetupData) -> BaseInterfaceHolder {
        // Create a headless ComInterface first
        let com_interface = Rc::new(ComInterface {
            info: Rc::new(ComInterfaceInfo::init(
                ComInterfaceState::NotConnected,
                InterfaceProperties::default(),
            )),
            implementation: RefCell::new(None),
        });

        // Create the implementation using the factory function
        let implementation = BaseInterface {
            properties: setup_data.properties,
            on_send: setup_data.on_send_callback,
            com_interface: com_interface.clone(),
        };
        com_interface.initialize(Box::new(implementation));

        BaseInterfaceHolder {
            sender: HashMap::new(),
            com_interface,
        }
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

    fn create_and_init_socket(
        &mut self,
        direction: InterfaceDirection,
    ) -> (ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>) {
        let (uuid, sender) = self
            .com_interface
            .socket_manager()
            .lock()
            .unwrap()
            .create_and_init_socket(direction, 1);
        (uuid, sender)
    }

    /// Registers and initializes a new socket with the given endpoint and direction
    /// Returns the socket UUID and a sender to send data to the socket
    pub fn register_new_socket_with_endpoint(
        &mut self,
        direction: InterfaceDirection,
        endpoint: Endpoint,
    ) -> (ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>) {
        let (socket_uuid, sender) = self.create_and_init_socket(direction);

        self.com_interface
            .socket_manager()
            .lock()
            .unwrap()
            .register_socket_with_endpoint(socket_uuid.clone(), endpoint, 1)
            .unwrap();
        (socket_uuid, sender)
    }
}

impl ComInterfaceImplementation for BaseInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        self.on_send.as_ref()(block, socket_uuid)
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

#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct BaseInterfaceSetupData {
    pub properties: InterfaceProperties,
    pub on_send_callback: Box<OnSendCallback>,
}

impl BaseInterfaceSetupData {
    pub fn new(
        properties: InterfaceProperties,
        on_send_callback: Box<OnSendCallback>,
    ) -> Self {
        BaseInterfaceSetupData {
            properties,
            on_send_callback,
        }
    }
    pub fn with_callback(on_send_callback: Box<OnSendCallback>) -> Self {
        BaseInterfaceSetupData {
            properties: InterfaceProperties::default(),
            on_send_callback,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        network::com_interfaces::{
            com_interface::{
                properties::InterfaceProperties, state::ComInterfaceState,
            },
            default_com_interfaces::base_interface::{
                self, BaseInterfaceHolder, BaseInterfaceSetupData,
            },
        },
        utils::context::init_global_context,
    };

    #[tokio::test]
    pub async fn test_close() {
        init_global_context();
        // Create a new interface
        let base_interface =
            BaseInterfaceHolder::new(BaseInterfaceSetupData::new(
                InterfaceProperties::default(),
                Box::new(|_, _| Box::pin(async move { true })),
            ))
            .com_interface
            .clone();
        assert_eq!(
            base_interface.current_state(),
            ComInterfaceState::NotConnected
        );
        assert!(base_interface.properties().close_timestamp.is_none());

        // Open the interface
        base_interface.open().await;
        assert_eq!(
            base_interface.current_state(),
            ComInterfaceState::Connected
        );
        assert!(base_interface.properties().close_timestamp.is_none());

        // Close the interface
        assert!(base_interface.close().await);
        assert_eq!(
            base_interface.current_state(),
            ComInterfaceState::NotConnected
        );
        assert!(base_interface.properties().close_timestamp.is_some());
    }
}
