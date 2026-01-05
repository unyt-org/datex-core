use super::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::{
        ComInterfaceSocket, ComInterfaceSocketUUID, SocketState,
    },
};
use crate::runtime::AsyncContext;
use crate::serde::deserializer::from_value_container;
use crate::std_sync::Mutex;
use crate::stdlib::{any::Any, cell::Cell, collections::VecDeque, pin::Pin};
use crate::stdlib::{boxed::Box, future::Future, sync::Arc, vec::Vec};
use crate::task::UnboundedSender;
use crate::utils::{time::Time, uuid::UUID};
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use crate::{collections::HashMap, task::UnboundedReceiver};
use crate::{network::com_hub::ComHub, task::create_unbounded_channel};
use crate::{
    stdlib::{
        cell::RefCell,
        hash::{Hash, Hasher},
        rc::Rc,
        string::String,
    },
    task::spawn_with_panic_notify,
};
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;
use log::{debug, error, warn};
use serde::Deserialize;

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

#[derive(Debug)]
pub struct ComInterfaceStateWrapper {
    state: ComInterfaceState,
    event_sender: UnboundedSender<ComInterfaceEvent>,
}

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
    pub fn get(&self) -> ComInterfaceState {
        self.state
    }
    pub fn set(&mut self, new_state: ComInterfaceState) {
        self.state = new_state;
        let event = match new_state {
            ComInterfaceState::NotConnected => ComInterfaceEvent::NotConnected,
            ComInterfaceState::Connected => ComInterfaceEvent::Connected,
            ComInterfaceState::Destroyed => ComInterfaceEvent::Destroyed,
            ComInterfaceState::Connecting => return, // No event for connecting state
        };
        self.event_sender.start_send(event);
    }
}

impl ComInterfaceState {
    pub fn set(&mut self, new_state: ComInterfaceState) {
        *self = new_state;
    }
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
            .start_send(ComInterfaceSocketEvent::NewSocket(socket.clone()));
    }
    pub fn remove_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        self.sockets.remove(socket_uuid);
        self.socket_event_sender.start_send(
            ComInterfaceSocketEvent::RemovedSocket(socket_uuid.clone()),
        );
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
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct ComInterfaceInfo {
    pub outgoing_blocks_count: Cell<u32>,
    uuid: ComInterfaceUUID,
    pub state: Arc<Mutex<ComInterfaceStateWrapper>>,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    pub interface_properties: Option<InterfaceProperties>,

    socket_event_receiver: Option<UnboundedReceiver<ComInterfaceSocketEvent>>,
    interface_event_receiver: Option<UnboundedReceiver<ComInterfaceEvent>>,
}

impl Default for ComInterfaceInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl ComInterfaceInfo {
    pub fn new_with_state(state: ComInterfaceState) -> Self {
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
            interface_properties: None,
            socket_event_receiver: Some(socket_event_receiver),
            com_interface_sockets: Arc::new(Mutex::new(
                ComInterfaceSockets::new_with_sender(socket_event_sender),
            )),
        }
    }

    pub fn new() -> Self {
        Self::new_with_state(ComInterfaceState::NotConnected)
    }
    pub fn com_interface_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
    pub fn get_uuid(&self) -> &ComInterfaceUUID {
        &self.uuid
    }
    pub fn get_state(&self) -> ComInterfaceState {
        self.state.try_lock().unwrap().get()
    }
    pub fn set_state(&mut self, new_state: ComInterfaceState) {
        self.state.try_lock().unwrap().set(new_state);
    }
}

/// This macro is used to create a new opener function for the ComInterface that
/// returns a boolean indicating if the opener was successful or not.
/// The method shall be only called by the ComHub that doesn't know the
/// actual return value of the specific opener function.
#[macro_export]
macro_rules! set_opener {
    ($opener:ident) => {
        fn handle_open<'a>(
            &'a mut self,
        ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
            self.set_state($crate::network::com_interfaces::com_interface::ComInterfaceState::Connecting);
            Box::pin(async move {
                let res = self.$opener().await.is_ok();
                if res {
                    self.set_state($crate::network::com_interfaces::com_interface::ComInterfaceState::Connected);
                } else {
                    self.set_state($crate::network::com_interfaces::com_interface::ComInterfaceState::NotConnected);
                }
                res
            })
        }
    };
}

#[macro_export]
macro_rules! set_sync_opener {
    ($opener:ident) => {
        fn handle_open<'a>(
            &'a mut self,
        ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
            self.set_state(ComInterfaceState::Connecting);
            Box::pin(async move {
                let res = self.$opener().is_ok();
                if res {
                    self.set_state(ComInterfaceState::Connected);
                } else {
                    self.set_state(ComInterfaceState::NotConnected);
                }
                res
            })
        }
    };
}

// TODO #193 use procedural macros instead
#[macro_export]
macro_rules! delegate_com_interface_info {
    () => {
        fn get_uuid(&self) -> &$crate::network::com_interfaces::com_interface::ComInterfaceUUID {
            &self.info.get_uuid()
        }
        fn get_state(&self) -> $crate::network::com_interfaces::com_interface::ComInterfaceState {
            self.info.get_state()
        }
        fn set_state(&mut self, new_state: $crate::network::com_interfaces::com_interface::ComInterfaceState) {
            self.info.set_state(new_state);
        }
        fn get_info(&self) -> &ComInterfaceInfo {
            &self.info
        }
        fn get_info_mut(&mut self) -> &mut ComInterfaceInfo {
            &mut self.info
        }
        fn get_sockets(&self) -> Arc<Mutex<$crate::network::com_interfaces::com_interface::ComInterfaceSockets>> {
            self.info.com_interface_sockets().clone()
        }

        fn as_any(&self) -> &dyn datex_core::stdlib::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn datex_core::stdlib::any::Any {
            self
        }
        fn get_properties(&mut self) -> &InterfaceProperties {
            if self.get_info().interface_properties.is_some() {
                return self.get_info().interface_properties.as_ref().unwrap();
            } else {
                let new_properties = self.init_properties();
                let info = self.get_info_mut();
                info.interface_properties = Some(new_properties);
                info.interface_properties.as_ref().unwrap()
            }
        }
        fn get_properties_mut(&mut self) -> &mut InterfaceProperties {
            if self.get_info().interface_properties.is_some() {
                return self
                    .get_info_mut()
                    .interface_properties
                    .as_mut()
                    .unwrap();
            } else {
                let new_properties = self.init_properties();
                let info = self.get_info_mut();
                info.interface_properties = Some(new_properties);
                info.interface_properties.as_mut().unwrap()
            }
        }
    };
}

/// This trait can be implemented by any ComInterface impl that wants to
/// support a factory method for creating instances of the interface.
/// Example:
/// ```
/// # use core::cell::RefCell;
/// # use datex_core::stdlib::rc::Rc;
/// # use datex_core::network::com_interfaces::com_interface::{ComInterface, ComInterfaceError, ComInterfaceFactory};///
/// # use datex_core::network::com_interfaces::com_interface_properties::InterfaceProperties;///
/// use serde::{Deserialize, Serialize};
/// use datex_core::network::com_interfaces::default_com_interfaces::base_interface::BaseInterface;
///
/// #[derive(Serialize, Deserialize)]
/// struct BaseInterfaceSetupData {
///    pub example_data: String,
/// }
///
/// impl ComInterfaceFactory<BaseInterfaceSetupData> for BaseInterface {
///     fn create(setup_data: BaseInterfaceSetupData) -> Result<BaseInterface, ComInterfaceError> {
///         // ...
///         Ok(BaseInterface::new_with_name("example"))
///     }
///     fn get_default_properties() -> InterfaceProperties {
///         InterfaceProperties {
///             interface_type: "example".to_string(),
///             ..Default::default()
///         }
///     }
/// }
pub trait ComInterfaceFactory<T>
where
    Self: Sized + ComInterface,
    T: Deserialize<'static> + 'static,
{
    /// The factory method that is called from the ComHub on a registered interface
    /// to create a new instance of the interface.
    /// The setup data is passed as a ValueContainer and has to be downcasted
    fn factory(
        setup_data: ValueContainer,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComInterfaceError> {
        let data = from_value_container::<T>(setup_data);
        match data {
            Ok(init_data) => {
                let interface = Self::create(init_data);
                match interface {
                    Ok(interface) => Ok(Rc::new(RefCell::new(interface))),
                    Err(e) => Err(e),
                }
            }
            Err(e) => {
                error!("Failed to deserialize setup data: {e}");
                core::panic!("Invalid setup data for com interface factory")
            }
        }
    }

    /// Register the interface on which the factory is implemented
    /// on the given ComHub.
    fn register_on_com_hub(com_hub: &ComHub) {
        let interface_type = Self::get_default_properties().interface_type;
        com_hub.register_interface_factory(interface_type, Self::factory);
    }

    /// Create a new instance of the interface with the given setup data.
    /// If no instance could be created with the given setup data,
    /// None is returned.
    fn create(setup_data: T) -> Result<Self, ComInterfaceError>;

    /// Get the default interface properties for the interface.
    fn get_default_properties() -> InterfaceProperties;
}

// #[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
// pub async fn flush_outgoing_block_task(
//     interface: Rc<RefCell<dyn ComInterface>>,
//     socket_ref: Arc<Mutex<ComInterfaceSocket>>,
//     block: Vec<u8>,
//     uuid: ComInterfaceSocketUUID,
// ) {
//     // FIXME #194 borrow_mut across await point!
//     let has_been_send = interface.borrow_mut().send_block(&block, uuid).await;
//     interface
//         .borrow()
//         .get_info()
//         .outgoing_blocks_count
//         .update(|x| x - 1);
//     if !has_been_send {
//         debug!("Failed to send block");
//         socket_ref
//             .try_lock()
//             .unwrap()
//             .bytes_in_sender
//             .push_back(block);
//         core::panic!("Failed to send block");
//     }
// }

// pub fn flush_outgoing_blocks(
//     interface: Rc<RefCell<dyn ComInterface>>,
//     async_context: &AsyncContext,
// ) {
//     fn get_blocks(socket_ref: &Arc<Mutex<ComInterfaceSocket>>) -> Vec<Vec<u8>> {
//         let mut socket_mut = socket_ref.try_lock().unwrap();
//         let blocks: Vec<Vec<u8>> =
//             socket_mut.bytes_in_sender.drain(..).collect::<Vec<_>>();
//         blocks
//     }
//     let sockets = interface.borrow().get_sockets();
//     for socket_ref in sockets.try_lock().unwrap().sockets.values() {
//         let blocks = get_blocks(socket_ref);
//         let interface = interface.clone();
//         for block in blocks {
//             let interface = interface.clone();
//             let socket_ref = socket_ref.clone();
//             let uuid = socket_ref.try_lock().unwrap().uuid.clone();
//             interface
//                 .borrow()
//                 .get_info()
//                 .outgoing_blocks_count
//                 .update(|x| x + 1);
//             spawn_with_panic_notify(
//                 async_context,
//                 flush_outgoing_block_task(interface, socket_ref, block, uuid),
//             );
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum ComInterfaceEvent {
    Connected,
    NotConnected,
    Destroyed,
}

pub trait ComInterface: Any {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;

    fn as_any(&self) -> &dyn crate::stdlib::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn crate::stdlib::any::Any;

    fn init_properties(&self) -> InterfaceProperties;
    // TODO #195: no mut, wrap self.info in RefCell
    fn get_properties(&mut self) -> &InterfaceProperties;
    fn get_properties_mut(&mut self) -> &mut InterfaceProperties;
    fn get_uuid(&self) -> &ComInterfaceUUID;

    fn get_info(&self) -> &ComInterfaceInfo;
    fn get_info_mut(&mut self) -> &mut ComInterfaceInfo;

    fn get_state(&self) -> ComInterfaceState;
    fn set_state(&mut self, new_state: ComInterfaceState);

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;

    /// Destroy all sockets of the interface
    /// This will add the sockets to the deleted_sockets list
    /// to be consumed by the ComHub
    fn destroy_sockets(&mut self) {
        let sockets = self.get_sockets();
        let sockets = sockets.try_lock().unwrap();
        let uuids: Vec<ComInterfaceSocketUUID> =
            sockets.sockets.keys().cloned().collect();
        drop(sockets);
        for socket_uuid in uuids {
            self.remove_socket(&socket_uuid);
        }
    }

    fn take_socket_event_receiver(
        &mut self,
    ) -> UnboundedReceiver<ComInterfaceSocketEvent> {
        self.get_info_mut().socket_event_receiver.take().expect(
            "Socket event receiver has already been taken from this interface",
        )
    }

    /// Close the interface and free all resources.
    /// Has to be implemented by the interface and might be async.
    /// The state is set by the close that calls the handler function
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;

    pub fn take_interface_event_receiver(
        &mut self,
    ) -> UnboundedReceiver<ComInterfaceEvent> {
        self.interface_event_receiver.take().expect(
            "Interface event receiver has already been taken from this interface",
        )
    }

    /// Public API to close the interface and clean up all sockets.
    /// This will set the state to `NotConnected` or `Destroyed` depending on
    /// if the interface could be closed or not.
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let uuid = self.get_uuid().clone();
        if self.get_state().is_destroyed_or_not_connected() {
            warn!("Interface {uuid} is already closed. Not closing again.");
            return Box::pin(async move { false });
        }
        Box::pin(async move {
            let ok = self.handle_close().await;
            if ok {
                debug!("Successfully closed interface {uuid}");
                self.set_state(ComInterfaceState::NotConnected);
            } else {
                error!("Error while closing interface {uuid}");
                // If the interface could not be closed, we set it to destroyed
                // to make sure it is cleaned up and not left in a dangling state.

                // When we can't close an interface, we won't reconnect it
                self.set_state(ComInterfaceState::Destroyed);
            }

            // Remove the sockets from the interface socket list
            // to notify ComHub routing logic
            self.destroy_sockets();

            // Update the close timestamp for interfaces that support reconnect
            // This is used to determine when the interface shall be reopened
            if ok && self.get_properties().shall_reconnect() {
                let time = Time::now();
                let properties = self.get_properties_mut();
                properties.close_timestamp = Some(time);
            }
            ok
        })
    }

    /// Public API to destroy the interface and free all resources.
    fn handle_destroy<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        if self.get_state().is_destroyed() {
            core::panic!(
                "Interface {} is already destroyed. Not destroying again.",
                self.get_uuid()
            );
        }
        Box::pin(async move {
            self.handle_close().await;
            self.destroy_sockets();
            self.set_state(ComInterfaceState::Destroyed);
        })
    }

    fn handle_open<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;

    // Add new socket to the interface (not registered yet)
    fn add_socket(&self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        let mut sockets =
            self.get_info().com_interface_sockets.try_lock().unwrap();
        sockets.add_socket(socket);
    }

    // Remove socket from the interface
    fn remove_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        let mut sockets =
            self.get_info().com_interface_sockets.try_lock().unwrap();
        let socket = sockets.get_socket_by_uuid(socket_uuid);
        socket.unwrap().try_lock().unwrap().state = SocketState::Destroyed;
        sockets.remove_socket(socket_uuid);
    }

    // Called when an endpoint is known for a specific socket (called by ComHub)
    fn register_socket_endpoint(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), ComInterfaceError> {
        let mut sockets =
            self.get_info().com_interface_sockets.try_lock().unwrap();
        sockets.register_socket_endpoint(socket_uuid, endpoint, distance)
    }

    fn get_channel_factor(&self) -> u32 {
        let properties = self.init_properties();
        properties.max_bandwidth / properties.round_trip_time.as_millis() as u32
    }

    fn init_socket(
        &self,
        direction: InterfaceDirection,
        channel_factor: u32,
    ) -> ComInterfaceSocket {
        ComInterfaceSocket::init(
            self.get_uuid().clone(),
            direction,
            channel_factor,
        )
    }

    fn init_socket_default(&self) -> ComInterfaceSocket {
        ComInterfaceSocket::init(
            self.get_uuid().clone(),
            self.init_properties().direction,
            self.get_channel_factor(),
        )
    }
}

impl PartialEq for dyn ComInterface {
    fn eq(&self, other: &Self) -> bool {
        self.get_uuid() == other.get_uuid()
    }
}
impl Eq for dyn ComInterface {}

impl Hash for dyn ComInterface {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let uuid = self.get_uuid();
        uuid.hash(state);
    }
}
