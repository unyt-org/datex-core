use crate::network::com_interfaces::com_interface::ComInterface;

use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::{
    ComInterfaceFactory, ComInterfaceImplementation,
};
use crate::network::com_interfaces::com_interface::properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::stdlib::boxed::Box;
use crate::stdlib::cell::RefCell;
use crate::stdlib::pin::Pin;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::ToString;
use crate::task::UnboundedSender;
use crate::values::core_values::endpoint::Endpoint;
use core::future::Future;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::time::Duration;

/// A simple local loopback interface that puts outgoing data
/// back into the incoming queue.
pub struct LocalLoopbackInterface {
    sender: RefCell<UnboundedSender<Vec<u8>>>,
}

impl ComInterfaceImplementation for LocalLoopbackInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        self.sender.borrow_mut().start_send(block.to_vec()).unwrap();
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
        com_interface: Rc<ComInterface>,
    ) -> Result<Self, ComInterfaceError> {
        // directly create a socket and register it
        let (socket_uuid, mut sender) = com_interface
            .socket_manager()
            .lock()
            .unwrap()
            .create_and_init_socket(InterfaceDirection::InOut, 1);
        com_interface
            .socket_manager()
            .lock()
            .unwrap()
            .register_socket_with_endpoint(socket_uuid, Endpoint::LOCAL, 1)?;

        Ok(LocalLoopbackInterface {
            sender: RefCell::new(sender),
        })
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties::default()
    }
}
