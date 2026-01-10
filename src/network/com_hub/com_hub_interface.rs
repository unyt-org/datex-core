use crate::network::com_hub::managers::interface_manager::{
    ComInterfaceImplementationFactoryFn, InterfaceManager,
};
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::stdlib::string::String;
use crate::stdlib::{cell::RefCell, rc::Rc};
use crate::task::{UnboundedReceiver, spawn_with_panic_notify};
use core::prelude::rust_2024::*;
use core::result::Result;

use crate::network::com_hub::{ComHub, ComHubError, InterfacePriority};
use crate::network::com_interfaces::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::{
    ComInterfaceEvent, ComInterfaceUUID,
};
use crate::values::value_container::ValueContainer;

/// Interface management methods
impl ComHub {
    /// Registers a new interface factory for the given interface type
    pub fn register_interface_factory(
        &self,
        interface_type: String,
        factory: ComInterfaceImplementationFactoryFn,
    ) {
        self.interface_manager
            .borrow_mut()
            .register_interface_factory(interface_type, factory);
    }

    /// Adds a new interface to the ComHub
    pub fn add_interface(
        &mut self,
        interface: Rc<ComInterface>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        self.interface_manager
            .borrow_mut()
            .add_interface(interface.clone(), priority)?;

        // handle socket events
        self.handle_interface_socket_events(interface.clone());
        // handle interface events
        self.handle_interface_events(interface);
        Ok(())
    }

    /// Opens the interface if not already opened, and adds it to the manager
    pub async fn open_and_add_interface(
        &self,
        interface: Rc<ComInterface>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        self.interface_manager
            .borrow_mut()
            .open_and_add_interface(interface.clone(), priority)
            .await?;

        // handle socket events
        self.handle_interface_socket_events(interface.clone());
        // handle interface events
        self.handle_interface_events(interface);
        Ok(())
    }

    /// Internal method to handle interface events
    fn handle_interface_events(&self, interface: Rc<ComInterface>) {
        let interface_event_receiver =
            interface.take_interface_event_receiver();
        let uuid = interface.uuid().clone();
        spawn_with_panic_notify(
            &self.async_context,
            handle_interface_events(
                uuid,
                interface_event_receiver,
                self.interface_manager.clone(),
            ),
        );
    }

    /// Returns the com interface for a given socket UUID
    /// The interface and socket must be registered in the ComHub,
    /// otherwise a panic will be triggered
    pub(crate) fn dyn_interface_for_socket_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Rc<ComInterface> {
        let socket_manager = self.socket_manager.borrow();
        let socket = socket_manager.get_socket_by_uuid(socket_uuid);
        self.interface_manager
            .borrow()
            .dyn_interface_by_uuid(&socket.interface_uuid)
    }

    /// Creates a new interface of the given type with the provided setup data
    pub async fn create_interface(
        &self,
        interface_type: &str,
        setup_data: ValueContainer,
        priority: InterfacePriority,
    ) -> Result<Rc<ComInterface>, ComHubError> {
        self.interface_manager
            .borrow_mut()
            .create_interface(interface_type, setup_data, priority)
            .await
    }

    pub async fn remove_interface(
        &self,
        interface_uuid: ComInterfaceUUID,
    ) -> Result<(), ComHubError> {
        self.interface_manager
            .borrow_mut()
            .remove_interface(interface_uuid)
            .await
    }

    pub fn has_interface(&self, interface_uuid: &ComInterfaceUUID) -> bool {
        self.interface_manager
            .borrow()
            .has_interface(interface_uuid)
    }
}

async fn handle_interface_events(
    uuid: ComInterfaceUUID,
    mut receiver_queue: UnboundedReceiver<ComInterfaceEvent>,
    interface_manager: Rc<RefCell<InterfaceManager>>,
) {
    while let Some(event) = receiver_queue.next().await {
        interface_manager
            .borrow_mut()
            .handle_interface_event(&uuid, event);
    }
}
