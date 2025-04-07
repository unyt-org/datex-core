use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use super::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::ComInterfaceSocket,
};
use crate::stdlib::fmt::Display;
use crate::stdlib::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
};
use crate::utils::uuid::UUID;
use log::debug;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComInterfaceUUID(pub UUID);
impl Display for ComInterfaceUUID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ComInterface({})", self.0)
    }
}
#[derive(Debug)]
pub enum ComInterfaceError {
    ConnectionError,
    SendError,
    ReceiveError,
}

pub trait ComInterface {
    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket);
    fn get_properties(&self) -> InterfaceProperties;
    fn get_sockets(&self) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>;
    fn connect(&mut self) -> Result<(), ComInterfaceError>;
    fn get_uuid(&self) -> ComInterfaceUUID;

    fn add_socket(&self, socket: Rc<RefCell<ComInterfaceSocket>>) {
        let sockets = self.get_sockets();
        sockets.borrow_mut().push(socket);
    }

    fn get_channel_factor(&self) -> u32 {
        let properties = self.get_properties();
        properties.max_bandwidth / properties.round_trip_time.as_millis() as u32
    }

    fn flush_outgoing_blocks(&mut self) {
        for socket_mut in self.get_sockets().borrow().iter() {
            let mut socket_mut = socket_mut.borrow_mut();
            let blocks: Vec<Vec<u8>> =
                socket_mut.send_queue.drain(..).collect::<Vec<_>>();

            debug!("Flushing {} blocks", blocks.len());
            debug!("Socket: {:?}", socket_mut.uuid);

            for block in blocks {
                self.send_block(&block, &socket_mut);
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
            channel_factor
        )
    }

    fn create_socket_default(
        &self,
        receive_queue: Arc<Mutex<VecDeque<u8>>>
    ) -> ComInterfaceSocket {
        ComInterfaceSocket::new_with_receive_queue(
            self.get_uuid().clone(),
            receive_queue,
            self.get_properties().direction,
            self.get_channel_factor()
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
