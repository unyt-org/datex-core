use super::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
};
use crate::stdlib::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
};
use crate::utils::uuid::UUID;
use crate::{datex_values::Endpoint, stdlib::fmt::Display};
use futures_util::future::join_all;
use log::{debug, info};
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    pin::Pin,
};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComInterfaceUUID(pub UUID);
impl Display for ComInterfaceUUID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ComInterface({})", self.0)
    }
}
#[derive(Debug)]
pub enum ComInterfaceError {
    SocketNotFound,
    SocketAlreadyExists,
    ConnectionError,
    SendError,
    ReceiveError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComInterfaceState {
    Created,
    Connecting,
    Connected,
    Closing,
    Closed,
}

impl ComInterfaceState {
    pub fn is_open(&self) -> bool {
        matches!(self, ComInterfaceState::Connected)
    }
    pub fn is_closed(&self) -> bool {
        matches!(self, ComInterfaceState::Closed)
    }
    pub fn is_opening(&self) -> bool {
        matches!(self, ComInterfaceState::Connecting)
    }
    pub fn is_closing(&self) -> bool {
        matches!(self, ComInterfaceState::Closing)
    }
    pub fn is_created(&self) -> bool {
        matches!(self, ComInterfaceState::Created)
    }
    pub fn is_connecting(&self) -> bool {
        matches!(
            self,
            ComInterfaceState::Connecting | ComInterfaceState::Connected
        )
    }
    pub fn is_disconnecting(&self) -> bool {
        matches!(self, ComInterfaceState::Closing | ComInterfaceState::Closed)
    }
    pub fn set_state(&mut self, new_state: ComInterfaceState) {
        *self = new_state;
    }
}

#[derive(Debug, Default)]
pub struct ComInterfaceSockets {
    pub sockets:
        HashMap<ComInterfaceSocketUUID, Arc<Mutex<ComInterfaceSocket>>>,
    pub socket_registrations: VecDeque<(ComInterfaceSocketUUID, u8, Endpoint)>,
    pub new_sockets: VecDeque<Arc<Mutex<ComInterfaceSocket>>>,
    pub deleted_sockets: VecDeque<ComInterfaceSocketUUID>,
}

impl ComInterfaceSockets {
    pub fn add_socket(&mut self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        let uuid = socket.lock().unwrap().uuid.clone();
        self.sockets.insert(uuid.clone(), socket.clone());
        self.new_sockets.push_back(socket);
        debug!("Socket added: {uuid}");
    }
    pub fn remove_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        self.sockets.remove(socket_uuid);
        self.deleted_sockets.push_back(socket_uuid.clone());
        debug!("Socket removed: {socket_uuid:?}");
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
            let mut socket = socket.unwrap().lock().unwrap();
            if socket.direct_endpoint.is_none() {
                socket.direct_endpoint = Some(endpoint.clone());
            }
        }

        debug!("Socket registered: {socket_uuid} {endpoint}");

        self.socket_registrations.push_back((
            socket_uuid,
            distance,
            endpoint.clone(),
        ));
        Ok(())
    }
}

pub struct ComInterfaceInfo {
    uuid: ComInterfaceUUID,
    state: Arc<Mutex<ComInterfaceState>>,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    pub interface_properties: Option<InterfaceProperties>,
}

impl Default for ComInterfaceInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl ComInterfaceInfo {
    pub fn new_with_state(state: ComInterfaceState) -> Self {
        Self {
            uuid: ComInterfaceUUID(UUID::new()),
            state: Arc::new(Mutex::new(state)),
            interface_properties: None,
            com_interface_sockets: Arc::new(Mutex::new(
                ComInterfaceSockets::default(),
            )),
        }
    }
    pub fn new() -> Self {
        Self {
            uuid: ComInterfaceUUID(UUID::new()),
            state: Arc::new(Mutex::new(ComInterfaceState::Created)),
            interface_properties: None,
            com_interface_sockets: Arc::new(Mutex::new(
                ComInterfaceSockets::default(),
            )),
        }
    }
    pub fn com_interface_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
    pub fn get_uuid(&self) -> &ComInterfaceUUID {
        &self.uuid
    }
    pub fn get_state(&self) -> Arc<Mutex<ComInterfaceState>> {
        self.state.clone()
    }
    pub fn set_state(&mut self, new_state: ComInterfaceState) {
        self.state.lock().unwrap().clone_from(&new_state);
    }
}
#[macro_export]
macro_rules! delegate_com_interface_info {
    () => {
        fn get_uuid(&self) -> &ComInterfaceUUID {
            &self.info.get_uuid()
        }
        fn get_state(&self) -> ComInterfaceState {
            self.info.get_state().lock().unwrap().clone()
        }
        fn set_state(&mut self, new_state: ComInterfaceState) {
            self.info.set_state(new_state);
        }
        fn get_info(&self) -> &ComInterfaceInfo {
            &self.info
        }
        fn get_info_mut(&mut self) -> &mut ComInterfaceInfo {
            &mut self.info
        }
        fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
            self.info.com_interface_sockets().clone()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
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
    };
}

pub trait ComInterface: Any {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    fn init_properties(&self) -> InterfaceProperties;
    fn get_properties(&mut self) -> &InterfaceProperties;
    fn get_uuid(&self) -> &ComInterfaceUUID;

    fn get_info(&self) -> &ComInterfaceInfo;
    fn get_info_mut(&mut self) -> &mut ComInterfaceInfo;

    fn get_state(&self) -> ComInterfaceState;
    fn set_state(&mut self, new_state: ComInterfaceState);

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;

    // Destroy the interface and free all resources after it has been cleaned up
    fn destroy_sockets(&mut self) {
        info!("destroy_sockets");
        let sockets = self.get_sockets();
        let sockets = sockets.lock().unwrap();
        let uuids: Vec<ComInterfaceSocketUUID> =
            sockets.sockets.keys().cloned().collect();
        drop(sockets);
        for socket_uuid in uuids {
            self.remove_socket(&socket_uuid);
        }
        self.set_state(ComInterfaceState::Closed);
    }

    /// Close the interface and free all resources.
    /// Has to be implemented by the interface and might be async.
    /// Make sure to call destroy_sockets() after the interface is closed.
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>>;

    // Add new socket to the interface (not registered yet)
    fn add_socket(&self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        let mut sockets = self.get_info().com_interface_sockets.lock().unwrap();
        sockets.add_socket(socket);
    }

    // Remove socket from the interface
    fn remove_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        let mut sockets = self.get_info().com_interface_sockets.lock().unwrap();
        sockets.remove_socket(socket_uuid);
    }

    // Called when an endpoint is known for a specific socket (called by ComHub)
    fn register_socket_endpoint(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), ComInterfaceError> {
        let mut sockets = self.get_info().com_interface_sockets.lock().unwrap();
        sockets.register_socket_endpoint(socket_uuid, endpoint, distance)
    }

    fn get_channel_factor(&self) -> u32 {
        let properties = self.init_properties();
        properties.max_bandwidth / properties.round_trip_time.as_millis() as u32
    }

    fn flush_outgoing_blocks<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        fn get_blocks(
            socket_ref: &Arc<Mutex<ComInterfaceSocket>>,
        ) -> Vec<Vec<u8>> {
            let mut socket_mut = socket_ref.lock().unwrap();
            let blocks: Vec<Vec<u8>> =
                socket_mut.send_queue.drain(..).collect::<Vec<_>>();
            blocks
        }

        Box::pin(async move {
            let sockets = self.get_sockets();
            let shared_self = &Rc::new(RefCell::new(self));
            join_all(
                // Iterate over all sockets
                sockets
                    .lock()
                    .unwrap()
                    .sockets
                    .values()
                    .map(|socket_ref| {
                        // Get all blocks of the socket
                        let blocks = get_blocks(socket_ref);

                        // Iterate over all blocks for a socket
                        blocks.into_iter().map(|block| {
                            // Send the block
                            let socket_ref = socket_ref.clone();
                            Box::pin(async move {
                                let mut socket_borrow =
                                    socket_ref.lock().unwrap();

                                // socket will return a boolean indicating of a block could be sent
                                let has_been_send = shared_self
                                    .clone()
                                    .borrow_mut()
                                    .send_block(
                                        &block,
                                        socket_borrow.uuid.clone(),
                                    )
                                    .await;

                                // If the block could not be sent, push it back to the send queue to be sent later
                                if !has_been_send {
                                    debug!("Failed to send block");
                                    socket_borrow.send_queue.push_back(block);
                                }
                            })
                        })
                    })
                    .flatten(),
            )
            .await;
        })
    }

    fn create_socket(
        &self,
        receive_queue: Arc<Mutex<VecDeque<u8>>>,
        direction: InterfaceDirection,
        channel_factor: u32,
    ) -> ComInterfaceSocket {
        ComInterfaceSocket::new_with_receive_queue(
            self.get_uuid().clone(),
            receive_queue,
            direction,
            channel_factor,
        )
    }

    fn create_socket_default(
        &self,
        receive_queue: Arc<Mutex<VecDeque<u8>>>,
    ) -> ComInterfaceSocket {
        ComInterfaceSocket::new_with_receive_queue(
            self.get_uuid().clone(),
            receive_queue,
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
