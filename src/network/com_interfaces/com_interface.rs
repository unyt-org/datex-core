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
use log::debug;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

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
    fn send_block(&mut self, block: &[u8], socket: Option<&ComInterfaceSocket>);
    fn get_properties(&self) -> InterfaceProperties;

    fn get_sockets(&self) -> Rc<RefCell<ComInterfaceSockets>>;

    fn connect(&mut self) -> Result<(), ComInterfaceError>;
    fn get_uuid(&self) -> ComInterfaceUUID;

    fn add_socket(&self, socket: Rc<RefCell<ComInterfaceSocket>>) {
        let sockets = self.get_sockets();
        let mut sockets = sockets.borrow_mut();
        sockets.new_sockets.push_back(socket.clone());
        sockets
            .sockets
            .insert(socket.borrow().uuid.clone(), socket.clone());
        debug!("Socket added: {:?}", socket.borrow().uuid);
    }

    fn remove_socket(&mut self, socket: &ComInterfaceSocket) {
        let sockets = self.get_sockets();
        let mut sockets = sockets.borrow_mut();

        sockets.deleted_sockets.push_back(socket.uuid.clone());
        sockets.sockets.remove(&socket.uuid);
        debug!("Socket removed: {:?}", socket.uuid);
    }

    fn register_socket_endpoint(
        &mut self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u32,
    ) -> Result<(), ComInterfaceError> {
        let sockets = self.get_sockets();
        let mut sockets = sockets.borrow_mut();

        if sockets.sockets.get(&socket_uuid).is_none() {
            return Err(ComInterfaceError::SocketNotFound);
        }
        debug!("Socket registered: {:?}", socket_uuid);

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

    fn flush_outgoing_blocks(&mut self) {
        for socket_ref in self.get_sockets().borrow().sockets.values() {
            let blocks = {
                let mut socket_mut = socket_ref.borrow_mut();
                let blocks: Vec<Vec<u8>> =
                    socket_mut.send_queue.drain(..).collect::<Vec<_>>();

                debug!("Flushing {} blocks", blocks.len());
                debug!("Socket: {:?}", socket_mut.uuid);
                blocks
            };
            for block in blocks {
                self.send_block(&block, Some(&socket_ref.borrow()));
            }
        }
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
