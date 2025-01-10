use super::super::com_interface::ComInterface;

pub struct TCPClientInterface {}

impl ComInterface for TCPClientInterface {
    fn send_block(&mut self, _block: &[u8]) -> () {
        todo!()
    }

    fn get_properties(
        &self,
    ) -> crate::network::com_interfaces::com_interface_properties::InterfaceProperties {
        todo!()
    }
}
