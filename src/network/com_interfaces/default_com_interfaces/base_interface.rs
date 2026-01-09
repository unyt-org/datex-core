use core::prelude::rust_2024::*;
use core::result::Result;
use log::error;

use super::super::com_interface_old::ComInterfaceOld;
use crate::network::com_hub::errors::ComHubError;
use crate::network::com_interfaces::com_interface::{ComInterfaceError, ComInterfaceInfo, ComInterfaceState, ComInterfaceSockets, ComInterface};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
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
use std::cell::RefCell;
use std::rc::Rc;
use serde::{Deserialize, Serialize};

pub type OnSendCallback = dyn Fn(&[u8], ComInterfaceSocketUUID) -> Pin<Box<dyn Future<Output = bool>>>
    + 'static;

pub struct BaseInterface {
    on_send: Option<Box<OnSendCallback>>,
    properties: InterfaceProperties,
    com_interface: Rc<RefCell<ComInterface>>,
}


use datex_macros::{com_interface, create_opener};
use strum::Display;
use thiserror::Error;
use crate::network::com_interfaces::com_interface_implementation::ComInterfaceImplementation;
use crate::network::com_interfaces::com_interface_implementation::ComInterfaceFactory;

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

    pub fn register_new_socket(
        &mut self,
        direction: InterfaceDirection,
    ) -> ComInterfaceSocketUUID {
        let mut interface = self
            .com_interface
            .borrow_mut();
        let socket =
            ComInterfaceSocket::init(interface.uuid().clone(), direction, 1);
        let socket_uuid = socket.uuid.clone();
        let socket = Arc::new(Mutex::new(socket));
        interface.add_socket(socket);
        socket_uuid
    }
    pub fn register_new_socket_with_endpoint(
        &mut self,
        direction: InterfaceDirection,
        endpoint: Endpoint,
    ) -> ComInterfaceSocketUUID {
        let socket_uuid = self.register_new_socket(direction);
        let mut interface = self
            .com_interface
            .borrow_mut();
        interface.register_socket_endpoint(socket_uuid.clone(), endpoint, 1)
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
        let interface = self
            .com_interface
            .borrow();
        match interface.get_socket_by_uuid(&receiver_socket_uuid) {
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

impl ComInterfaceImplementation for BaseInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let interface = self
            .com_interface
            .borrow();
        if !interface.has_socket_with_uuid(&socket_uuid) {
            return Box::pin(async move { false });
        }
        if let Some(on_send) = &self.on_send {
            on_send(block, socket_uuid)
        } else {
            Box::pin(async move { false })
        }
    }

    fn get_properties(&self) -> InterfaceProperties {
        self.properties.clone()
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }

    fn handle_open<'a>(&'a mut self) -> Pin<Box<dyn Future<Output=bool> + 'a>> {
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct BaseInterfaceSetupData(pub InterfaceProperties);

impl ComInterfaceFactory for BaseInterface {
    type SetupData = BaseInterfaceSetupData;

    fn create(
        setup_data: BaseInterfaceSetupData,
        com_interface: Rc<RefCell<ComInterface>>
    ) -> Result<BaseInterface, ComInterfaceError> {
        Ok(
            BaseInterface {
                properties: setup_data.0,
                on_send: None,
                com_interface,
            }
        )
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties::default()
    }
}
