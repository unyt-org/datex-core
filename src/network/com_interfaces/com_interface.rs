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

#[derive(Debug, Default)]
pub struct ComInterfaceSockets {
    pub sockets:
        HashMap<ComInterfaceSocketUUID, Rc<RefCell<ComInterfaceSocket>>>,
    pub socket_registrations: VecDeque<(ComInterfaceSocketUUID, u32, Endpoint)>,
    pub new_sockets: VecDeque<Rc<RefCell<ComInterfaceSocket>>>,
    pub deleted_sockets: VecDeque<ComInterfaceSocketUUID>,
}

pub trait ComInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: Option<ComInterfaceSocketUUID>,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;

    fn get_properties(&self) -> InterfaceProperties;
    fn get_uuid<'a>(&'a self) -> &'a ComInterfaceUUID;

    fn get_sockets(&self) -> Rc<RefCell<ComInterfaceSockets>>;

    // Opens the interface and prepares it for communication.
    fn open<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), ComInterfaceError>> + 'a>>;

    // Destroy the interface and free all resources.
    fn close(&mut self) -> Result<(), ComInterfaceError> {
        // FIXME
        Ok(())
    }

    // Add new socket to the interface (not registered yet)
    fn add_socket(&self, socket: Rc<RefCell<ComInterfaceSocket>>) {
        let sockets = self.get_sockets();
        let mut sockets = sockets.borrow_mut();
        sockets.new_sockets.push_back(socket.clone());
        sockets
            .sockets
            .insert(socket.borrow().uuid.clone(), socket.clone());
        debug!("Socket added: {}", socket.borrow().uuid);
    }

    // Remove socket from the interface
    fn remove_socket(&mut self, socket: &ComInterfaceSocket) {
        let sockets = self.get_sockets();
        let mut sockets = sockets.borrow_mut();

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
        let mut sockets = sockets.borrow_mut();

        let socket = sockets.sockets.get(&socket_uuid);
        if socket.is_none() {
            return Err(ComInterfaceError::SocketNotFound);
        }
        {
            let mut socket = socket.unwrap().borrow_mut();
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
        let properties = self.get_properties();
        properties.max_bandwidth / properties.round_trip_time.as_millis() as u32
    }

    fn flush_outgoing_blocks<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        fn get_blocks(
            socket_ref: &Rc<RefCell<ComInterfaceSocket>>,
        ) -> Vec<Vec<u8>> {
            let mut socket_mut = socket_ref.borrow_mut();
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
                    .borrow()
                    .sockets
                    .values()
                    .into_iter()
                    .map(|socket_ref| {
                        // Get all blocks of the socket
                        let blocks = get_blocks(socket_ref);

                        // Iterate over all blocks for a socket
                        blocks.into_iter().map(|block| {
                            // Send the block
                            let socket_ref = socket_ref.clone();
                            Box::pin(async move {
                                let socket_borrow = socket_ref.borrow();

                                // socket will return a boolean indicating of a block could be sent
                                let has_been_send = shared_self
                                    .clone()
                                    .borrow_mut()
                                    .send_block(
                                        &block,
                                        Some(socket_borrow.uuid.clone()),
                                    )
                                    .await;

                                // If the block could not be sent, push it back to the send queue to be sent later
                                if !has_been_send {
                                    debug!("Failed to send block");
                                    socket_ref
                                        .borrow_mut()
                                        .send_queue
                                        .push_back(block);
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
            self.get_properties().direction,
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
