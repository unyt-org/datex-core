use crate::network::com_hub::ComHub;
use std::{
    cell::RefCell, hash::{Hash, Hasher}, rc::Rc
};

use super::com_interface_properties::InterfaceProperties;

pub trait ComInterface {
    fn send_block(&mut self, block: &[u8]) -> ();
    fn get_properties(&self) -> InterfaceProperties;
    fn get_com_interface_handler(&self) -> &ComInterfaceHandler;
}

pub struct ComInterfaceHandler {
    pub com_hub: Rc<RefCell<ComHub>>,
}
impl ComInterfaceHandler {
    pub fn new(com_hub: Rc<RefCell<ComHub>>) -> Self {
        ComInterfaceHandler { com_hub }
    }
    fn receive_block(&mut self, block: &[u8]) {
        self.com_hub.borrow_mut().receive_block(block);
    }
}

#[derive(Clone)]
pub struct ComInterfaceTrait {
    pub interface: Rc<dyn ComInterface>,
}

impl ComInterfaceTrait {
    pub fn new(inner: Rc<dyn ComInterface>) -> Self {
        ComInterfaceTrait { interface: inner }
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
