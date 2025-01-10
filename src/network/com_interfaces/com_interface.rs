use super::com_interface_properties::InterfaceProperties;

pub trait ComInterface {
    fn send_block(&mut self, block: &[u8]) -> ();
    fn get_properties(&self) -> InterfaceProperties;
}
