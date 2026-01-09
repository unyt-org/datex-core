use crate::network::com_interfaces::com_interface::ComInterface;

use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::{
    ComInterfaceFactory, ComInterfaceImplementation,
};
use crate::network::com_interfaces::com_interface::properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface::socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::std_sync::Mutex;
use crate::stdlib::boxed::Box;
use crate::stdlib::pin::Pin;
use crate::stdlib::string::ToString;
use crate::stdlib::sync::Arc;
use crate::values::core_values::endpoint::Endpoint;
use core::future::Future;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::time::Duration;
use std::cell::RefCell;
use std::rc::Rc;

/// A simple local loopback interface that puts outgoing data
/// back into the incoming queue.
pub struct LocalLoopbackInterface {
    socket: Arc<Mutex<ComInterfaceSocket>>,
}

impl ComInterfaceImplementation for LocalLoopbackInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        self.socket
            .try_lock()
            .unwrap()
            .bytes_in_sender
            .try_lock()
            .unwrap()
            .start_send(block.to_vec())
            .unwrap();
        Box::pin(async { true })
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "local".to_string(),
            channel: "local".to_string(),
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            ..InterfaceProperties::default()
        }
    }
    fn handle_close<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }

    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }
}

impl ComInterfaceFactory for LocalLoopbackInterface {
    type SetupData = ();

    fn create(
        _setup_data: Self::SetupData,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<Self, ComInterfaceError> {
        let mut com_interface = com_interface.borrow_mut();
        let socket = Arc::new(Mutex::new(ComInterfaceSocket::init(
            com_interface.uuid().clone(),
            InterfaceDirection::InOut,
            1,
        )));
        let socket_uuid = socket.try_lock().unwrap().uuid.clone();
        com_interface.add_socket(socket.clone());
        com_interface.register_socket_endpoint(
            socket_uuid,
            Endpoint::LOCAL,
            1,
        )?;

        Ok(LocalLoopbackInterface { socket })
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties::default()
    }
}
