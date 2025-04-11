use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;

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
        socket: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        todo!()
    }

    fn get_uuid(&self) -> &ComInterfaceUUID {
        todo!()
    }

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        todo!()
    }
}
