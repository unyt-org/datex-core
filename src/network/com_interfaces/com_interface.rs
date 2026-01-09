use crate::stdlib::any::Any;
use core::cell::Cell;
use crate::collections::HashMap;
use core::fmt::Display;
use core::pin::Pin;
use crate::stdlib::sync::{Arc, Mutex};
use core::time::Duration;
use std::cell::RefCell;
use std::rc::Rc;
use binrw::error::CustomError;
use log::debug;
use crate::network::com_hub::ComInterfaceImplementationFactoryFn;
use crate::network::com_interfaces::com_interface_implementation::{ComInterfaceFactory, ComInterfaceImplementation};
use crate::network::com_interfaces::com_interface_properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID, SocketState};
use crate::task::{create_unbounded_channel, UnboundedReceiver, UnboundedSender};
use crate::utils::uuid::UUID;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;

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

#[derive(Debug, Clone, PartialEq)]
pub enum ComInterfaceError {
    SocketNotFound,
    SocketAlreadyExists,
    ConnectionError,
    SendError,
    ReceiveError,
    InvalidSetupData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::EnumIs)]
pub enum ComInterfaceState {
    NotConnected,
    Connected,
    Connecting,
    Destroyed,
}

#[derive(Debug, Clone)]
pub enum ComInterfaceEvent {
    Connected,
    NotConnected,
    Destroyed,
}


#[derive(Debug)]
pub struct ComInterfaceStateWrapper {
    state: ComInterfaceState,
    event_sender: UnboundedSender<ComInterfaceEvent>,
}

/// Wrapper around ComInterfaceState that sends events on state changes
impl ComInterfaceStateWrapper {
    pub fn new(
        state: ComInterfaceState,
        event_sender: UnboundedSender<ComInterfaceEvent>,
    ) -> Self {
        ComInterfaceStateWrapper {
            state,
            event_sender,
        }
    }

    /// Get the current state
    pub fn get(&self) -> ComInterfaceState {
        self.state
    }

    /// Set a new state and send the corresponding event
    pub fn set(&mut self, new_state: ComInterfaceState) {
        self.state = new_state;
        let event = match new_state {
            ComInterfaceState::NotConnected => ComInterfaceEvent::NotConnected,
            ComInterfaceState::Connected => ComInterfaceEvent::Connected,
            ComInterfaceState::Destroyed => ComInterfaceEvent::Destroyed,
            ComInterfaceState::Connecting => return, // No event for connecting state
        };
        let _ = self.event_sender.start_send(event);
    }
}

impl ComInterfaceState {
    pub fn is_destroyed_or_not_connected(&self) -> bool {
        core::matches!(
            self,
            ComInterfaceState::Destroyed | ComInterfaceState::NotConnected
        )
    }
}

#[derive(Debug)]
pub struct ComInterfaceSockets {
    pub sockets:
        HashMap<ComInterfaceSocketUUID, Arc<Mutex<ComInterfaceSocket>>>,
    socket_event_sender: UnboundedSender<ComInterfaceSocketEvent>,
}

impl ComInterfaceSockets {
    pub fn new_with_sender(
        sender: UnboundedSender<ComInterfaceSocketEvent>,
    ) -> Self {
        ComInterfaceSockets {
            sockets: HashMap::new(),
            socket_event_sender: sender,
        }
    }
}

#[derive(Debug)]
pub enum ComInterfaceSocketEvent {
    NewSocket(Arc<Mutex<ComInterfaceSocket>>),
    RemovedSocket(ComInterfaceSocketUUID),
    RegisteredSocket(ComInterfaceSocketUUID, i8, Endpoint),
}

impl ComInterfaceSockets {
    pub fn add_socket(&mut self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        {
            let mut socket_mut = socket.try_lock().unwrap();
            let uuid = socket_mut.uuid.clone();
            socket_mut.state = SocketState::Open;
            self.sockets.insert(uuid.clone(), socket.clone());
            debug!("Socket added: {uuid}");
        }
        self.socket_event_sender
            .start_send(ComInterfaceSocketEvent::NewSocket(socket.clone()))
            .unwrap();
    }
    pub fn remove_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        self.sockets.remove(socket_uuid);
        self.socket_event_sender.start_send(
            ComInterfaceSocketEvent::RemovedSocket(socket_uuid.clone()),
        ).unwrap();
        if let Some(socket) = self.sockets.get(socket_uuid) {
            socket.try_lock().unwrap().state = SocketState::Destroyed;
        }
    }
    pub fn get_socket_by_uuid(
        &self,
        uuid: &ComInterfaceSocketUUID,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        self.sockets.get(uuid).cloned()
    }

    pub fn register_socket_endpoint(
        &mut self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), ComInterfaceError> {
        let socket = self.sockets.get(&socket_uuid);
        if socket.is_none() {
            return Err(ComInterfaceError::SocketNotFound);
        }
        {
            let mut socket = socket.unwrap().try_lock().unwrap();
            if socket.direct_endpoint.is_none() {
                socket.direct_endpoint = Some(endpoint.clone());
            }
        }

        debug!("Socket registered: {socket_uuid} {endpoint}");
        self.socket_event_sender.start_send(
            ComInterfaceSocketEvent::RegisteredSocket(
                socket_uuid,
                distance as i8,
                endpoint.clone(),
            ),
        ).unwrap();
        Ok(())
    }
}



#[derive(Debug)]
pub struct ComInterfaceInfo {
    pub outgoing_blocks_count: Cell<u32>,
    uuid: ComInterfaceUUID,
    pub state: Arc<Mutex<ComInterfaceStateWrapper>>,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    pub interface_properties: InterfaceProperties,

    socket_event_receiver: Option<UnboundedReceiver<ComInterfaceSocketEvent>>,
    interface_event_receiver: Option<UnboundedReceiver<ComInterfaceEvent>>,
}

impl ComInterfaceInfo {
    pub fn new_with_state_and_properties(state: ComInterfaceState, interface_properties: InterfaceProperties) -> Self {
        let (socket_event_sender, socket_event_receiver) =
            create_unbounded_channel::<ComInterfaceSocketEvent>();
        let (interface_event_sender, interface_event_receiver) =
            create_unbounded_channel::<ComInterfaceEvent>();
        Self {
            outgoing_blocks_count: Cell::new(0),
            uuid: ComInterfaceUUID(UUID::new()),
            state: Arc::new(Mutex::new(ComInterfaceStateWrapper::new(
                state,
                interface_event_sender,
            ))),
            interface_event_receiver: Some(interface_event_receiver),
            interface_properties,
            socket_event_receiver: Some(socket_event_receiver),
            com_interface_sockets: Arc::new(Mutex::new(
                ComInterfaceSockets::new_with_sender(socket_event_sender),
            )),
        }
    }

    pub fn com_interface_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
    pub fn uuid(&self) -> &ComInterfaceUUID {
        &self.uuid
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
        implementation: Box<dyn ComInterfaceImplementation>,
        info: ComInterfaceInfo,
    },
}


impl ComInterface {
    /// Creates a new ComInterface with a specified implementation as returned by the factory function
    pub fn create_from_factory_fn(
        factory_fn: ComInterfaceImplementationFactoryFn,
        setup_data: ValueContainer,
    ) -> Result<Rc<RefCell<ComInterface>>, ComInterfaceError>{
        // Create a headless ComInterface first
        let com_interface = Rc::new(RefCell::new(
            ComInterface::Headless {
                info: Some(ComInterfaceInfo::new_with_state_and_properties(
                    ComInterfaceState::NotConnected,
                    InterfaceProperties::default(),
                ))
            }
        ));

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
        let com_interface = Rc::new(RefCell::new(
            ComInterface::Headless {
                info: Some(ComInterfaceInfo::new_with_state_and_properties(
                    ComInterfaceState::NotConnected,
                    InterfaceProperties::default(),
                )),
            }
        ));

        // Create the implementation using the factory function
        let implementation = T::create(setup_data, com_interface.clone())?;
        com_interface.borrow_mut().initialize(Box::new(implementation));
        Ok(com_interface)
    }

    /// Initializes a headless ComInterface with the provided implementation
    /// and upgrades it to an Initialized state.
    /// This can only be done once on a headless interface and will panic if attempted on an already initialized interface.
    fn initialize(
        &mut self,
        implementation: Box<dyn ComInterfaceImplementation>,
    ) {
        match self {
            ComInterface::Headless { info } => {
                *self = ComInterface::Initialized {
                    implementation,
                    info: info.take().expect("ComInterfaceInfo should be present when initializing"),
                };
            }
            ComInterface::Initialized { .. } => {
                panic!("ComInterface is already initialized with an implementation");
            }
        }
    }

    pub fn uuid(&self) -> &ComInterfaceUUID {
        match self {
            ComInterface::Headless { info } => info.as_ref().unwrap().uuid(),
            ComInterface::Initialized { info, .. } => info.uuid(),
        }
    }

    pub fn state(&self) -> ComInterfaceState {
        match self {
            ComInterface::Headless { info } => info.as_ref().unwrap().state(),
            ComInterface::Initialized { info, .. } => info.state(),
        }
    }

    pub fn set_state(&mut self, new_state: ComInterfaceState) {
        match self {
            ComInterface::Headless { info } => info.as_mut().unwrap().set_state(new_state),
            ComInterface::Initialized { info, .. } => info.set_state(new_state),
        }
    }

    pub fn properties(&self) -> &InterfaceProperties {
        match self {
            ComInterface::Headless { info } => &info.as_ref().unwrap().interface_properties,
            ComInterface::Initialized { info, .. } => &info.interface_properties,
        }
    }

    pub fn properties_mut(&mut self) -> &mut InterfaceProperties {
        match self {
            ComInterface::Headless { info } => &mut info.as_mut().unwrap().interface_properties,
            ComInterface::Initialized { info, .. } => &mut info.interface_properties,
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

    pub fn add_socket(
        &mut self,
        socket: Arc<Mutex<ComInterfaceSocket>>,
    ) {
        let info = match self {
            ComInterface::Headless { info } => info.as_ref().unwrap(),
            ComInterface::Initialized { info, .. } => info,
        };
        info.com_interface_sockets
            .try_lock()
            .unwrap()
            .add_socket(socket);
    }

    pub fn register_socket_endpoint(
        &mut self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), ComInterfaceError> {
        let info = match self {
            ComInterface::Headless { info } => info.as_ref().unwrap(),
            ComInterface::Initialized { info, .. } => info,
        };
        info.com_interface_sockets
            .try_lock()
            .unwrap()
            .register_socket_endpoint(socket_uuid, endpoint, distance)
    }

    pub fn get_socket_by_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        let info = match self {
            ComInterface::Headless { info } => info.as_ref().unwrap(),
            ComInterface::Initialized { info, .. } => info,
        };
        info.com_interface_sockets
            .try_lock()
            .unwrap()
            .get_socket_by_uuid(socket_uuid)
    }

    pub fn has_socket_with_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> bool {
        let info = match self {
            ComInterface::Headless { info } => info.as_ref().unwrap(),
            ComInterface::Initialized { info, .. } => info,
        };
        info.com_interface_sockets
            .try_lock()
            .unwrap()
            .sockets
            .contains_key(socket_uuid)
    }

    // Attempts to get a reference to the inner implementation
    // as a specific concrete type T.
    fn try_get_as_implementation<T: ComInterfaceImplementation>(
        &self,
    ) -> Option<&T> {
        match self {
            ComInterface::Headless { .. } => None,
            ComInterface::Initialized { implementation, .. } => {
                match implementation.as_any_ref().downcast_ref::<T>() {
                    Some(concrete_impl) => Some(concrete_impl),
                    None => None,
                }
            }
        }
    }

    pub fn take_interface_event_receiver(&mut self) -> UnboundedReceiver<ComInterfaceEvent> {
        let maybe_receiver = match self {
            ComInterface::Headless { info } => info.as_mut().unwrap().interface_event_receiver.take(),
            ComInterface::Initialized { info, .. } => info.interface_event_receiver.take(),
        };
        maybe_receiver.expect("Interface event receiver has already been taken")
    }

    pub fn take_socket_event_receiver(&mut self) -> UnboundedReceiver<ComInterfaceSocketEvent> {
        let maybe_receiver = match self {
            ComInterface::Headless { info } => info.as_mut().unwrap().socket_event_receiver.take(),
            ComInterface::Initialized { info, .. } => info.socket_event_receiver.take(),
        };
        maybe_receiver.expect("Socket event receiver has already been taken")
    }
}