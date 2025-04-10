use std::future::Future;
use std::pin::Pin;

use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocket;

use super::super::com_interface::ComInterface;

pub struct TCPClientInterface {}

impl ComInterface for TCPClientInterface {
    fn get_properties(
        &self,
    ) -> crate::network::com_interfaces::com_interface_properties::InterfaceProperties{
        todo!()
    }
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: Option<&ComInterfaceSocket>,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        todo!()
    }

    fn open(&mut self) -> Result<(), ComInterfaceError> {
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
