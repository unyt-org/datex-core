
use std::{cell::RefCell, collections::VecDeque, rc::Rc, sync::{Arc, Mutex}};

use anyhow::Result;
use async_trait::async_trait;

use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocket;

use super::super::com_interface::ComInterface;

pub struct TCPClientInterface {}

#[async_trait]
impl ComInterface for TCPClientInterface {


    fn send_block(&mut self, _block: &[u8], socket: &ComInterfaceSocket) -> () {
        todo!()
    }

    fn get_properties(
        &self,
    ) -> crate::network::com_interfaces::com_interface_properties::InterfaceProperties {
        todo!()
    }

    fn get_sockets(
        &self,
    ) -> Rc<RefCell<Vec<Arc<Mutex<ComInterfaceSocket>>>>>
    {
        todo!()
    }
      
    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }
}
