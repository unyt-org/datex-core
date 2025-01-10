use crate::network::com_interfaces::com_interface::ComInterfaceHandler;

use super::super::com_interface::ComInterface;

pub struct TCPClientInterface {
    handler: ComInterfaceHandler,
}

impl ComInterface for TCPClientInterface {
    fn send_block(&mut self, _block: &[u8]) -> () {
        todo!()
    }

    fn get_properties(
        &self,
    ) -> crate::network::com_interfaces::com_interface_properties::InterfaceProperties {
        todo!()
    }

    fn get_com_interface_handler(&self) -> &ComInterfaceHandler {
        &self.handler
    }
}
