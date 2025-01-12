use std::{
    cell::RefCell,
    collections::VecDeque,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::{Arc, Mutex},
};

use super::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::ComInterfaceSocket,
};

pub trait ComInterface {
    fn send_block(&mut self, block: &[u8], socket: ComInterfaceSocket) -> ();
    fn get_receive_queue(
        &mut self,
        socket: ComInterfaceSocket,
    ) -> Arc<Mutex<VecDeque<u8>>> {
        socket.get_receive_queue()
    }
    fn get_properties(&self) -> InterfaceProperties;

    fn get_sockets(&self) -> Vec<ComInterfaceSocket> {
        vec![]
    }
}

#[derive(Clone)]
pub struct ComInterfaceTrait {
    pub interface: Rc<RefCell<dyn ComInterface>>,
}

impl ComInterfaceTrait {
    pub fn new(inner: Rc<RefCell<dyn ComInterface>>) -> Self {
        ComInterfaceTrait { interface: inner }
    }

    pub fn send_block(&mut self, block: &[u8], socket: ComInterfaceSocket) {
        let interface = &mut self.interface;
        let mut interface_mut = interface.borrow_mut();
        interface_mut.send_block(block, socket);
    }

    pub fn get_receive_queue(
        &mut self,
        socket: ComInterfaceSocket,
    ) -> Arc<Mutex<VecDeque<u8>>> {
        let interface = &mut self.interface;
        let mut interface_mut = interface.borrow_mut();
        interface_mut.get_receive_queue(socket)
    }

    pub fn get_properties(&self) -> InterfaceProperties {
        let interface = &self.interface;
        let interface_ref = interface.borrow();
        interface_ref.get_properties()
    }

    pub fn get_sockets(&self) -> Vec<ComInterfaceSocket> {
        let interface = &self.interface;
        let interface_ref = interface.borrow();
        interface_ref.get_sockets()
    }

    pub fn get_channel_factor(&self, socket: ComInterfaceSocket) -> u32 {
        let interface = &self.interface.borrow();
        let properties = interface.get_properties();
        return properties.bandwidth / properties.latency;
    }

    pub fn can_send(&self, socket: ComInterfaceSocket) -> bool {
        let interface = &self.interface.borrow();
        let properties = interface.get_properties();
        return properties.direction == InterfaceDirection::OUT
            || properties.direction == InterfaceDirection::IN_OUT;
    }

    pub fn can_receive(&self, socket: ComInterfaceSocket) -> bool {
        let interface = &self.interface.borrow();
        let properties = interface.get_properties();
        return properties.direction == InterfaceDirection::IN
            || properties.direction == InterfaceDirection::IN_OUT;
    }
}

impl PartialEq for ComInterfaceTrait {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.interface, &other.interface)
    }
}

impl Eq for ComInterfaceTrait {}

impl Hash for ComInterfaceTrait {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let addr = Rc::as_ptr(&self.interface);
        addr.hash(state);
    }
}
