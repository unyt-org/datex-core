use crate::{network::com_interfaces::com_interface::{error::ComInterfaceError, implementation::ComInterfaceImplementation, properties::InterfaceDirection, state::ComInterfaceState}, stdlib::{cell::RefCell, rc::Rc}};

use log::info;

use crate::{
    collections::HashMap,
    network::{
        com_hub::{ComHubError, InterfacePriority},
        com_interfaces::{
        },
    },
    values::value_container::ValueContainer,
};
use crate::network::com_interfaces::com_interface::{ComInterface, ComInterfaceUUID, ComInterfaceEvent};

type InterfaceMap = HashMap<
    ComInterfaceUUID,
    (Rc<RefCell<ComInterface>>, InterfacePriority),
>;

pub type ComInterfaceImplementationFactoryFn =
    fn(
        setup_data: ValueContainer,
        interface: Rc<RefCell<ComInterface>>,
    ) -> Result<Box<dyn ComInterfaceImplementation>, ComInterfaceError>;

#[derive(Default)]
pub struct InterfaceManager {
    /// a list of all available interface factories, keyed by their interface type
    pub interface_factories: HashMap<String, ComInterfaceImplementationFactoryFn>,

    /// a list of all available interfaces, keyed by their UUID
    pub interfaces: InterfaceMap,
}

/// Manages the registered interfaces and their factories
/// Allows creating, adding, removing and querying interfaces
/// Also handles interface events (lifecycle management)
impl InterfaceManager {
    /// Registers a new interface factory for a specific interface implementation.
    /// This allows the ComHub to create new instances of the interface on demand.
    pub fn register_interface_factory(
        &mut self,
        interface_type: String,
        factory: ComInterfaceImplementationFactoryFn,
    ) {
        self.interface_factories.insert(interface_type, factory);
    }

    /// Creates a new interface instance using the registered factory
    /// for the specified interface type if it exists.
    /// The interface is opened and added to the ComHub.
    pub async fn create_interface(
        &mut self,
        interface_type: &str,
        setup_data: ValueContainer,
        priority: InterfacePriority,
    ) -> Result<Rc<RefCell<ComInterface>>, ComHubError> {
        info!("creating interface {interface_type}");
        if let Some(factory) = self.interface_factories.get(interface_type) {
            let interface = ComInterface::create_from_factory_fn(
                factory.clone(),
                setup_data
            ).map_err(ComHubError::InterfaceError)?;

            self.open_and_add_interface(interface.clone(), priority)
                .await
                .map(|_| interface)
        } else {
            Err(ComHubError::InterfaceTypeDoesNotExist)
        }
    }
    
    /// Checks if the interface with the given UUID exists in the manager
    pub fn has_interface(&self, interface_uuid: &ComInterfaceUUID) -> bool {
        self.interfaces.contains_key(interface_uuid)
    }

    /// Returns the com interface for a given UUID
    /// The interface is returned as a dynamic trait object
    pub fn try_dyn_interface_by_uuid(
        &self,
        uuid: &ComInterfaceUUID,
    ) -> Option<Rc<RefCell<ComInterface>>> {
        self.interfaces
            .get(uuid)
            .map(|(interface, _)| interface.clone())
    }

    /// Returns the com interface for a given UUID
    /// The interface must be registered in the ComHub,
    /// otherwise a panic will be triggered
    pub(crate) fn dyn_interface_by_uuid(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Rc<RefCell<ComInterface>> {
        self.try_dyn_interface_by_uuid(interface_uuid)
            .unwrap_or_else(|| {
                core::panic!("Interface for uuid {interface_uuid} not found")
            })
    }

    /// Opens the interface if not already opened, and adds it to the manager
    pub async fn open_and_add_interface(
        &mut self,
        interface: Rc<RefCell<ComInterface>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        let current_state = interface.borrow().state().lock().unwrap().get();
        if current_state != ComInterfaceState::Connected {
            // If interface is not connected, open it
            // and wait for it to be connected
            // FIXME #240: borrow_mut across await point
            if !(interface.borrow_mut().handle_open().await) {
                return Err(ComHubError::InterfaceOpenFailed);
            }
        }
        self.add_interface(interface.clone(), priority)
    }

    /// Adds an interface to the manager, checking for duplicates
    pub fn add_interface(
        &mut self,
        interface: Rc<RefCell<ComInterface>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        let uuid = interface.borrow().uuid().clone();
        if self.interfaces.contains_key(&uuid) {
            return Err(ComHubError::InterfaceAlreadyExists);
        }

        // make sure the interface can send if a priority is set
        if priority != InterfacePriority::None
            && interface.borrow_mut().properties().direction
                == InterfaceDirection::In
        {
            return Err(
                ComHubError::InvalidInterfaceDirectionForFallbackInterface,
            );
        }

        self.interfaces.insert(uuid, (interface.clone(), priority));
        Ok(())
    }

    /// Returns the priority of the interface with the given UUID
    pub fn interface_priority(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Option<InterfacePriority> {
        self.interfaces
            .get(interface_uuid)
            .map(|(_, priority)| *priority)
    }

    /// User can proactively remove an interface from the hub.
    /// This will destroy the interface and it's sockets (perform deep cleanup)
    pub async fn remove_interface(
        &mut self,
        interface_uuid: ComInterfaceUUID,
    ) -> Result<(), ComHubError> {
        info!("Removing interface {interface_uuid}");
        let interface = self
            .interfaces
            .get_mut(&interface_uuid.clone())
            .ok_or(ComHubError::InterfaceDoesNotExist)?
            .0
            .clone();
        {
            // Async close the interface (stop tasks, server, cleanup internal data)
            // FIXME #176: borrow_mut should not be used here
            let mut interface = interface.borrow_mut();
            interface.handle_destroy().await;
        }

        self.cleanup_interface(&interface_uuid)
            .ok_or(ComHubError::InterfaceDoesNotExist)?;

        Ok(())
    }

    /// The internal cleanup function that removes the interface from the hub
    /// and disabled the default interface if it was set to this interface
    fn cleanup_interface(
        &mut self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Option<Rc<RefCell<ComInterface>>> {
        Some(self.interfaces.remove(&interface_uuid).or(None)?.0)
    }

    /// Handles interface events received from interfaces
    pub fn handle_interface_event(
        &mut self,
        interface_uuid: &ComInterfaceUUID,
        event: ComInterfaceEvent,
    ) {
        match event {
            ComInterfaceEvent::Destroyed => {
                // FIXME should probably do more cleanup here, but this was what com hub did before
                self.cleanup_interface(interface_uuid);
            }
            _ => {}
        }
    }
}
