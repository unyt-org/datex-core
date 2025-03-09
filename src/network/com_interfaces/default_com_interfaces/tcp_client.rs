use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocket;
use anyhow::Result;

use super::super::com_interface::ComInterface;

pub struct TCPClientInterface {}

impl ComInterface for TCPClientInterface {
    fn send_block(&mut self, _block: &[u8], socket: &ComInterfaceSocket) -> () {
        todo!()
    }

    fn get_properties(
        &self,
    ) -> crate::network::com_interfaces::com_interface_properties::InterfaceProperties{
        todo!()
    }

    fn get_sockets(
        &self,
    ) -> std::rc::Rc<
        std::cell::RefCell<
            Vec<std::rc::Rc<std::cell::RefCell<ComInterfaceSocket>>>,
        >,
    > {
        todo!()
    }

    fn connect(&mut self) -> Result<()> {
        todo!()
    }

    fn get_uuid(&self) -> ComInterfaceUUID {
        todo!()
    }
}
