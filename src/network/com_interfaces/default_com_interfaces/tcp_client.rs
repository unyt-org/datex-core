use anyhow::Result;

use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocket;

use super::super::com_interface::ComInterface;

pub struct TCPClientInterface {}

impl ComInterface for TCPClientInterface {
  fn connect(&mut self) -> Result<()> {
    todo!()
  }

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
  
  fn get_uuid(&self) -> String {
        todo!()
    }
}
