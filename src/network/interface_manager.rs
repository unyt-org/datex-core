use std::{cell::RefCell, rc::Rc};

use log::info;

use crate::{
    collections::HashMap,
    network::{
        com_hub::{ComHubError, InterfacePriority},
        com_interfaces::{
            com_interface::{
                ComInterface, ComInterfaceError, ComInterfaceEvent,
                ComInterfaceState, ComInterfaceUUID,
            },
            com_interface_properties::InterfaceDirection,
        },
    },
    values::value_container::ValueContainer,
};

type InterfaceMap = HashMap<
    ComInterfaceUUID,
    (Rc<RefCell<dyn ComInterface>>, InterfacePriority),
>;

pub type ComInterfaceFactoryFn =
    fn(
        setup_data: ValueContainer,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComInterfaceError>;

#[derive(Default)]
pub struct InterfaceManager {
    /// a list of all available interface factories, keyed by their interface type
    pub interface_factories: HashMap<String, ComInterfaceFactoryFn>,

    /// a list of all available interfaces, keyed by their UUID
    pub interfaces: InterfaceMap,
}

impl InterfaceManager {
    /// Registers a new interface factory for a specific interface implementation.
    /// This allows the ComHub to create new instances of the interface on demand.
    pub fn register_interface_factory(
        &mut self,
        interface_type: String,
        factory: ComInterfaceFactoryFn,
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
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComHubError> {
        info!("creating interface {interface_type}");
        if let Some(factory) = self.interface_factories.get(interface_type) {
            let interface =
                factory(setup_data).map_err(ComHubError::InterfaceError)?;

            self.open_and_add_interface(interface.clone(), priority)
                .await
                .map(|_| interface)
        } else {
            Err(ComHubError::InterfaceTypeDoesNotExist)
        }
    }

    /// Returns the com interface for a given UUID
    /// The interface is downcasted to the specific type T
    pub fn interface_by_uuid<T: ComInterface>(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Option<Rc<RefCell<T>>> {
        InterfaceManager::try_downcast(
            self.interfaces.get(interface_uuid)?.0.clone(),
        )
    }

    /// Attempts to downcast a dynamic ComInterface trait object
    /// to a specific concrete type T.
    fn try_downcast<T: 'static>(
        input: Rc<RefCell<dyn ComInterface>>,
    ) -> Option<Rc<RefCell<T>>> {
        // Try to get a reference to the inner value
        if input.borrow().as_any().is::<T>() {
            // SAFETY: We're ensuring T is the correct type via the check
            let ptr = Rc::into_raw(input) as *const RefCell<T>;
            unsafe { Some(Rc::from_raw(ptr)) }
        } else {
            None
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
    ) -> Option<Rc<RefCell<dyn ComInterface>>> {
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
    ) -> Rc<RefCell<dyn ComInterface>> {
        self.try_dyn_interface_by_uuid(interface_uuid)
            .unwrap_or_else(|| {
                core::panic!("Interface for uuid {interface_uuid} not found")
            })
    }

    /// Opens the interface if not already opened, and adds it to the manager
    pub async fn open_and_add_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        if interface.borrow().get_state() != ComInterfaceState::Connected {
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
        interface: Rc<RefCell<dyn ComInterface>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        let uuid = interface.borrow().uuid().clone();
        if self.interfaces.contains_key(&uuid) {
            return Err(ComHubError::InterfaceAlreadyExists);
        }

        // make sure the interface can send if a priority is set
        if priority != InterfacePriority::None
            && interface.borrow_mut().get_properties().direction
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
    ) -> Option<Rc<RefCell<dyn ComInterface>>> {
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
