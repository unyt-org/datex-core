use std::{
  cell::RefCell,
  hash::{Hash, Hasher},
  rc::Rc,
};

use anyhow::Result;

use super::{
  com_interface_properties::{InterfaceDirection, InterfaceProperties},
  com_interface_socket::ComInterfaceSocket,
};

pub trait ComInterface {
  fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> ();
  fn get_properties(&self) -> InterfaceProperties;
  fn get_sockets(&self) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>;
  fn connect(&mut self) -> Result<()>;
}

#[derive(Clone)]
pub struct ComInterfaceTrait {
  pub interface: Rc<RefCell<dyn ComInterface>>,
}

impl ComInterfaceTrait {
  pub fn new(inner: Rc<RefCell<dyn ComInterface>>) -> Self {
    ComInterfaceTrait { interface: inner }
  }

  pub fn connect(&mut self) -> Result<()> {
    self.interface.borrow_mut().connect()
  }

  pub fn get_properties(&self) -> InterfaceProperties {
    let interface = &self.interface;
    let interface_ref = interface.borrow();
    interface_ref.get_properties()
  }

  pub fn get_sockets(
    &self,
  ) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>> {
    self.interface.borrow().get_sockets()
  }

  pub fn add_socket(&self, socket: Rc<RefCell<ComInterfaceSocket>>) {
    let sockets = self.get_sockets();
    sockets.borrow_mut().push(socket);
  }

  pub fn get_channel_factor(&self, socket: ComInterfaceSocket) -> u32 {
    let interface = &self.interface.borrow();
    let properties = interface.get_properties();
    return properties.bandwidth / properties.latency;
  }

  pub fn can_send(&self, socket: ComInterfaceSocket) -> bool {
    let interface = &self.interface.borrow();
    let properties = interface.get_properties();
    return properties.direction == InterfaceDirection::OUT
      || properties.direction == InterfaceDirection::IN_OUT;
  }

  pub fn can_receive(&self, socket: ComInterfaceSocket) -> bool {
    let interface = &self.interface.borrow();
    let properties = interface.get_properties();
    return properties.direction == InterfaceDirection::IN
      || properties.direction == InterfaceDirection::IN_OUT;
  }

  pub fn flush_outgoing_blocks(&self) {
    for socket_mut in self.get_sockets().borrow().iter() {
      let mut socket_mut = socket_mut.borrow_mut();
      let blocks: Vec<Vec<u8>> =
        socket_mut.send_queue.drain(..).collect::<Vec<_>>();

      println!("Flushing {} blocks", blocks.len());
      println!("Socket: {:?}", socket_mut.uuid);

      for block in blocks {
        let interface = &mut self.interface.borrow_mut();
        interface.send_block(&block, &socket_mut);
      }
    }
  }
}

impl PartialEq for ComInterfaceTrait {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.interface, &other.interface)
  }
}

impl Eq for ComInterfaceTrait {}

impl Hash for ComInterfaceTrait {
  fn hash<H: Hasher>(&self, state: &mut H) {
    let addr = Rc::as_ptr(&self.interface);
    addr.hash(state);
  }
}
