use std::{hash::{Hash, Hasher}, rc::Rc};
use super::com_interface_properties::InterfaceProperties;

pub trait ComInterface {
    fn send_block(&mut self, block: &[u8]) -> ();
    fn get_properties(&self) -> InterfaceProperties;
}


#[derive(Clone)]
pub struct ComInterfaceTrait {
    pub inner: Rc<dyn ComInterface>,
}

impl ComInterfaceTrait {
    pub fn new(inner: Rc<dyn ComInterface>) -> Self {
        ComInterfaceTrait { inner }
    }
}

impl PartialEq for ComInterfaceTrait {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ComInterfaceTrait {}

impl Hash for ComInterfaceTrait {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let addr = Rc::as_ptr(&self.inner);
        addr.hash(state);
    }
}
