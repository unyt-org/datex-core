use crate::collections::HashMap;
use crate::network::com_hub::ComInterfaceImplementationFactoryFn;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::{
    ComInterfaceFactory, ComInterfaceImpl, ComInterfaceImplementation,
};
use crate::network::com_interfaces::com_interface::properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface::socket::{
    ComInterfaceSocket, ComInterfaceSocketEvent, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::com_interface::socket_manager::ComInterfaceSocketManager;
use crate::network::com_interfaces::com_interface::state::{
    ComInterfaceState, ComInterfaceStateWrapper,
};

use crate::stdlib::any::Any;
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::stdlib::sync::MutexGuard;
use crate::stdlib::sync::{Arc, Mutex};
use crate::task::{
    UnboundedReceiver, UnboundedSender, create_unbounded_channel,
};
use crate::utils::once_consumer::OnceConsumer;
use crate::utils::uuid::UUID;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use binrw::error::CustomError;
use core::cell::Cell;
use core::fmt::Display;
use core::pin::Pin;
use core::time::Duration;
use log::debug;
pub mod error;
pub mod implementation;
pub mod properties;
pub mod socket;
pub mod socket_manager;
pub mod state;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComInterfaceUUID(pub UUID);
impl Display for ComInterfaceUUID {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::write!(f, "ComInterface({})", self.0)
    }
}

impl ComInterfaceUUID {
    pub fn from_string(uuid: String) -> Self {
        ComInterfaceUUID(UUID::from_string(uuid))
    }
}

#[derive(Debug, Clone)]
pub enum ComInterfaceEvent {
    Connected,
    NotConnected,
    Destroyed,
}

#[derive(Debug)]
pub struct ComInterfaceInfo {
    // Unique identifier
    pub uuid: ComInterfaceUUID,

    /// Connection state
    pub state: Arc<Mutex<ComInterfaceStateWrapper>>,

    /// Manager for sockets associated with this interface
    pub socket_manager: Arc<Mutex<ComInterfaceSocketManager>>,

    /// Details about the interface
    pub interface_properties: InterfaceProperties,

    /// Receiver for interface events (consumed by ComHub)
    socket_event_receiver:
        RefCell<OnceConsumer<UnboundedReceiver<ComInterfaceSocketEvent>>>,

    /// Receiver for interface events (consumed by ComHub)
    interface_event_receiver:
        RefCell<OnceConsumer<UnboundedReceiver<ComInterfaceEvent>>>,
}

impl ComInterfaceInfo {
    pub fn init(
        state: ComInterfaceState,
        interface_properties: InterfaceProperties,
    ) -> Self {
        let (socket_event_sender, socket_event_receiver) =
            create_unbounded_channel::<ComInterfaceSocketEvent>();
        let (interface_event_sender, interface_event_receiver) =
            create_unbounded_channel::<ComInterfaceEvent>();
        let uuid = ComInterfaceUUID(UUID::new());
        Self {
            state: Arc::new(Mutex::new(ComInterfaceStateWrapper::new(
                state,
                interface_event_sender,
            ))),
            socket_manager: Arc::new(Mutex::new(
                ComInterfaceSocketManager::new_with_sender(
                    uuid.clone(),
                    socket_event_sender,
                ),
            )),
            uuid,
            interface_event_receiver: RefCell::new(OnceConsumer::new(
                interface_event_receiver,
            )),
            interface_properties,
            socket_event_receiver: RefCell::new(OnceConsumer::new(
                socket_event_receiver,
            )),
        }
    }

    pub fn take_socket_event_receiver(
        &self,
    ) -> UnboundedReceiver<ComInterfaceSocketEvent> {
        self.socket_event_receiver.borrow_mut().consume()
    }
    pub fn take_interface_event_receiver(
        &self,
    ) -> UnboundedReceiver<ComInterfaceEvent> {
        self.interface_event_receiver.borrow_mut().consume()
    }

    pub fn state(&self) -> ComInterfaceState {
        self.state.try_lock().unwrap().get()
    }
    pub fn set_state(&mut self, new_state: ComInterfaceState) {
        self.state.try_lock().unwrap().set(new_state);
    }
}

/// A communication interface wrapper
/// which contains a concrete implementation of a com interface logic
pub enum ComInterface {
    Headless {
        info: Option<ComInterfaceInfo>,
    },
    Initialized {
        implementation: Box<dyn ComInterfaceImpl>,
        info: ComInterfaceInfo,
    },
}

impl ComInterface {
    /// Creates a new ComInterface with a specified implementation as returned by the factory function
    pub fn create_from_factory_fn(
        factory_fn: ComInterfaceImplementationFactoryFn,
        setup_data: ValueContainer,
    ) -> Result<Rc<RefCell<ComInterface>>, ComInterfaceError> {
        // Create a headless ComInterface first
        let com_interface = Rc::new(RefCell::new(ComInterface::Headless {
            info: Some(ComInterfaceInfo::init(
                ComInterfaceState::NotConnected,
                InterfaceProperties::default(),
            )),
        }));

        // Create the implementation using the factory function
        let implementation = factory_fn(setup_data, com_interface.clone())?;
        com_interface.borrow_mut().initialize(implementation);
        Ok(com_interface)
    }

    /// Creates a new ComInterface with the implementation of type T
    pub fn create_with_implementation<T>(
        setup_data: T::SetupData,
    ) -> Result<Rc<RefCell<ComInterface>>, ComInterfaceError>
    where
        T: ComInterfaceImplementation + ComInterfaceFactory,
    {
        // Create a headless ComInterface first
        let com_interface = Rc::new(RefCell::new(ComInterface::Headless {
            info: Some(ComInterfaceInfo::init(
                ComInterfaceState::NotConnected,
                InterfaceProperties::default(),
            )),
        }));

        // Create the implementation using the factory function
        let implementation = T::create(setup_data, com_interface.clone())?;
        com_interface
            .borrow_mut()
            .initialize(Box::new(implementation));
        Ok(com_interface)
    }

    pub fn implementation_mut<T: ComInterfaceImpl>(&mut self) -> &mut T {
        match self {
            ComInterface::Headless { .. } => {
                panic!(
                    "ComInterface is not initialized with an implementation"
                );
            }
            ComInterface::Initialized { implementation, .. } => implementation
                .as_mut()
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("Failed to downcast ComInterfaceImplementation"),
        }
    }

    /// Initializes a headless ComInterface with the provided implementation
    /// and upgrades it to an Initialized state.
    /// This can only be done once on a headless interface and will panic if attempted on an already initialized interface.
    fn initialize(&mut self, implementation: Box<dyn ComInterfaceImpl>) {
        match self {
            ComInterface::Headless { info } => {
                *self = ComInterface::Initialized {
                    implementation,
                    info: info.take().expect(
                        "ComInterfaceInfo should be present when initializing",
                    ),
                };
            }
            ComInterface::Initialized { .. } => {
                panic!(
                    "ComInterface is already initialized with an implementation"
                );
            }
        }
    }

    pub fn uuid(&self) -> &ComInterfaceUUID {
        match self {
            ComInterface::Headless { info } => &info.as_ref().unwrap().uuid,
            ComInterface::Initialized { info, .. } => &info.uuid,
        }
    }

    pub fn current_state(&self) -> ComInterfaceState {
        self.state().lock().unwrap().get()
    }

    pub fn state(&self) -> Arc<Mutex<ComInterfaceStateWrapper>> {
        match self {
            ComInterface::Headless { info } => {
                info.as_ref().unwrap().state.clone()
            }
            ComInterface::Initialized { info, .. } => info.state.clone(),
        }
    }

    pub fn set_state(&mut self, new_state: ComInterfaceState) {
        match self {
            ComInterface::Headless { info } => {
                info.as_mut().unwrap().set_state(new_state)
            }
            ComInterface::Initialized { info, .. } => info.set_state(new_state),
        }
    }

    pub fn properties(&self) -> &InterfaceProperties {
        match self {
            ComInterface::Headless { info } => {
                &info.as_ref().unwrap().interface_properties
            }
            ComInterface::Initialized { info, .. } => {
                &info.interface_properties
            }
        }
    }

    pub fn properties_mut(&mut self) -> &mut InterfaceProperties {
        match self {
            ComInterface::Headless { info } => {
                &mut info.as_mut().unwrap().interface_properties
            }
            ComInterface::Initialized { info, .. } => {
                &mut info.interface_properties
            }
        }
    }

    pub async fn send_block(
        &mut self,
        block: &[u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> bool {
        match self {
            ComInterface::Headless { .. } => {
                panic!("Cannot send block on headless ComInterface");
            }
            ComInterface::Initialized { implementation, .. } => {
                implementation.send_block(block, socket_uuid).await
            }
        }
    }

    pub async fn handle_open(&mut self) -> bool {
        match self {
            ComInterface::Headless { .. } => {
                panic!("Cannot open headless ComInterface");
            }
            ComInterface::Initialized { implementation, .. } => {
                implementation.handle_open().await
            }
        }
    }

    pub async fn handle_destroy(&mut self) -> bool {
        match self {
            ComInterface::Headless { .. } => {
                panic!("Cannot destroy headless ComInterface");
            }
            ComInterface::Initialized { implementation, .. } => {
                implementation.handle_close().await
            }
        }
    }

    pub fn info(&self) -> &ComInterfaceInfo {
        match self {
            ComInterface::Headless { info } => info.as_ref().unwrap(),
            ComInterface::Initialized { info, .. } => info,
        }
    }

    pub fn socket_manager(&self) -> Arc<Mutex<ComInterfaceSocketManager>> {
        self.info().socket_manager.clone()
    }

    pub fn take_interface_event_receiver(
        &mut self,
    ) -> UnboundedReceiver<ComInterfaceEvent> {
        match self {
            ComInterface::Headless { info } => {
                info.as_mut().unwrap().take_interface_event_receiver()
            }
            ComInterface::Initialized { info, .. } => {
                info.take_interface_event_receiver()
            }
        }
    }

    pub fn take_socket_event_receiver(
        &mut self,
    ) -> UnboundedReceiver<ComInterfaceSocketEvent> {
        match self {
            ComInterface::Headless { info } => {
                info.as_mut().unwrap().take_socket_event_receiver()
            }
            ComInterface::Initialized { info, .. } => {
                info.take_socket_event_receiver()
            }
        }
    }
}
