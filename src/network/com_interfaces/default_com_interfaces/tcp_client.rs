use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocket;

use super::super::com_interface::ComInterface;

pub struct TCPClientInterface {}

impl ComInterface for TCPClientInterface {
    fn send_block(
        &mut self,
        _block: &[u8],
        socket: Option<&ComInterfaceSocket>,
    ) {
        todo!()
    }

    fn get_properties(
        &self,
    ) -> crate::network::com_interfaces::com_interface_properties::InterfaceProperties{
        todo!()
    }

    fn connect(&mut self) -> Result<(), ComInterfaceError> {
        todo!()
    }

    fn get_uuid(&self) -> ComInterfaceUUID {
        todo!()
    }

    fn get_sockets(
        &self,
    ) -> std::rc::Rc<
        std::cell::RefCell<
            crate::network::com_interfaces::com_interface::ComInterfaceSockets,
        >,
    > {
        todo!()
    }
}
