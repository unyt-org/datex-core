use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
};
use std::fmt::Display;
use anyhow::Result;
use crate::crypto::uuid::generate_uuid;
use super::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::ComInterfaceSocket,
};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComInterfaceUUID(String);

impl ComInterfaceUUID {
    pub fn new() -> ComInterfaceUUID {
        ComInterfaceUUID(generate_uuid())
    }
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Default for ComInterfaceUUID {
    fn default() -> Self {
        ComInterfaceUUID("00000000-0000-0000-0000-000000000000".to_string())
    }
}

impl Display for ComInterfaceUUID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}



pub trait ComInterface {
    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> ();
    fn get_properties(&self) -> InterfaceProperties;
    fn get_sockets(&self) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>;
    fn connect(&mut self) -> Result<()>;
    fn get_uuid(&self) -> ComInterfaceUUID;

    fn add_socket(&self, socket: Rc<RefCell<ComInterfaceSocket>>) {
        let sockets = self.get_sockets();
        sockets.borrow_mut().push(socket);
    }

    fn get_channel_factor(&self, socket: ComInterfaceSocket) -> u32 {
        let properties = self.get_properties();
        return properties.max_bandwidth
            / properties.round_trip_time.as_millis() as u32;
    }

    fn can_send(&self, socket: ComInterfaceSocket) -> bool {
        let properties = self.get_properties();
        return properties.direction == InterfaceDirection::OUT
            || properties.direction == InterfaceDirection::IN_OUT;
    }

    fn can_receive(&self, socket: ComInterfaceSocket) -> bool {
        let properties = self.get_properties();
        return properties.direction == InterfaceDirection::IN
            || properties.direction == InterfaceDirection::IN_OUT;
    }

    fn flush_outgoing_blocks(&mut self) {
        for socket_mut in self.get_sockets().borrow().iter() {
            let mut socket_mut = socket_mut.borrow_mut();
            let blocks: Vec<Vec<u8>> =
                socket_mut.send_queue.drain(..).collect::<Vec<_>>();

            println!("Flushing {} blocks", blocks.len());
            println!("Socket: {:?}", socket_mut.uuid);

            for block in blocks {
                self.send_block(&block, &socket_mut);
            }
        }
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