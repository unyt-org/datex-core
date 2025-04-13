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
use log::debug;
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

#[derive(Debug)]
pub enum ComInterfaceState {
    Created,
    Opening,
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Default)]
pub struct ComInterfaceSockets {
    pub sockets:
        HashMap<ComInterfaceSocketUUID, Arc<Mutex<ComInterfaceSocket>>>,
    pub socket_registrations: VecDeque<(ComInterfaceSocketUUID, u32, Endpoint)>,
    pub new_sockets: VecDeque<Arc<Mutex<ComInterfaceSocket>>>,
    pub deleted_sockets: VecDeque<ComInterfaceSocketUUID>,
}

impl ComInterfaceSockets {
    pub fn add_socket(&mut self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        let uuid = socket.lock().unwrap().uuid.clone();
        self.sockets.insert(uuid, socket.clone());
        self.new_sockets.push_back(socket);
    }
    pub fn remove_socket(&mut self, socket: &ComInterfaceSocketUUID) {
        self.sockets.remove(socket);
        self.deleted_sockets.push_back(socket.clone());
    }
    pub fn get_socket_by_uuid(
        &self,
        uuid: &ComInterfaceSocketUUID,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        self.sockets.get(uuid).cloned()
    }
}

pub struct ComInterfaceInfo {
    state: ComInterfaceState,
    uuid: ComInterfaceUUID,
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    pub interface_properties: Option<InterfaceProperties>,
}

impl ComInterfaceInfo {
    pub fn new() -> Self {
        Self {
            uuid: ComInterfaceUUID(UUID::new()),
            state: ComInterfaceState::Created,
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
    pub fn get_state(&self) -> &ComInterfaceState {
        &self.state
    }
    pub fn set_state(&mut self, new_state: ComInterfaceState) {
        self.state = new_state;
    }
}
#[macro_export]
macro_rules! delegate_com_interface_info {
    () => {
        fn get_uuid(&self) -> &ComInterfaceUUID {
            &self.info.get_uuid()
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
    // fn get_socket_state_mut(&mut self) -> &mut ComInterfaceInfo;

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;

    // Destroy the interface and free all resources.
    fn close(&mut self) -> Result<(), ComInterfaceError> {
        // FIXME
        Ok(())
    }

    // Add new socket to the interface (not registered yet)
    fn add_socket(&self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        let sockets = self.get_sockets();
        let mut sockets = sockets.lock().unwrap();
        sockets.new_sockets.push_back(socket.clone());
        let uuid = socket.clone().lock().unwrap().uuid.clone();
        sockets.sockets.insert(uuid.clone(), socket.clone());
        debug!("Socket added: {}", uuid);
    }

    // Remove socket from the interface
    fn remove_socket(&mut self, socket: &ComInterfaceSocket) {
        let sockets = self.get_sockets();
        let mut sockets = sockets.lock().unwrap();

        sockets.deleted_sockets.push_back(socket.uuid.clone());
        sockets.sockets.remove(&socket.uuid);
        debug!("Socket removed: {:?}", socket.uuid);
    }

    // Called when a endpoint is known for a specific socket (called by ComHub)
    fn register_socket_endpoint(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u32,
    ) -> Result<(), ComInterfaceError> {
        let sockets = self.get_sockets();
        let mut sockets = sockets.lock().unwrap();

        let socket = sockets.sockets.get(&socket_uuid);
        if socket.is_none() {
            return Err(ComInterfaceError::SocketNotFound);
        }
        {
            let mut socket = socket.unwrap().lock().unwrap();
            if socket.direct_endpoint.is_none() {
                socket.direct_endpoint = Some(endpoint.clone());
            }
        }

        debug!("Socket registered: {} {}", socket_uuid, endpoint);

        sockets.socket_registrations.push_back((
            socket_uuid,
            distance,
            endpoint.clone(),
        ));
        Ok(())
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

            debug!("Flushing {} blocks", blocks.len());
            debug!("Socket: {:?}", socket_mut.uuid);
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
            debug!("Flushed all outgoing blocks");
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
