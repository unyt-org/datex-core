use std::{
  cell::RefCell, collections::VecDeque, rc::Rc, sync::{Arc, Mutex}
};

use crate::{
  crypto::{uuid::UUID}, datex_values::Endpoint, global::dxb_block::DXBBlock, runtime::Context, utils::logger::Logger
};

use super::block_collector::BlockCollector;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketState {
  Closed,
  Open,
  Error,
}

#[derive(Debug)]
pub struct ComInterfaceSocket {
  pub endpoint: Option<Endpoint>,
  pub is_connected: bool,
  pub is_open: bool,
  pub is_destroyed: bool,
  pub uuid: UUID<ComInterfaceSocket>,
  pub connection_timestamp: u64,
  pub receive_queue: Arc<Mutex<VecDeque<u8>>>,
  pub send_queue: VecDeque<Vec<u8>>,
  pub block_collector: BlockCollector,

  pub logger: Option<Logger>,
}

impl ComInterfaceSocket {
  pub fn get_receive_queue(&self) -> Arc<Mutex<VecDeque<u8>>> {
    self.receive_queue.clone()
  }

  pub fn get_incoming_block_queue(&self) -> &VecDeque<DXBBlock> {
    self.block_collector.get_block_queue()
  }

  pub fn collect_incoming_data(&mut self) {
    if let Some(logger) = &self.logger {
      logger.info(&format!("Collecting incoming data for {}", &self.uuid));
    }
    self.block_collector.update();
  }

  pub fn queue_outgoing_block(&mut self, block: &[u8]) {
    self.send_queue.push_back(block.to_vec());
  }

  pub fn empty() -> ComInterfaceSocket {
    ComInterfaceSocket::default()
  }
  pub fn new(
    context: Rc<RefCell<Context>>,
    logger: Option<Logger>,
  ) -> ComInterfaceSocket {
    let receive_queue = Arc::new(Mutex::new(VecDeque::new()));
    ComInterfaceSocket::new_with_receive_queue(
      context,
      receive_queue,
      logger,
    )
  }
 
  pub fn new_with_receive_queue(
    context: Rc<RefCell<Context>>,
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
    logger: Option<Logger>,
  ) -> ComInterfaceSocket {
    ComInterfaceSocket {
      logger: logger.clone(),
      receive_queue: receive_queue.clone(),
      block_collector: BlockCollector::new_with_logger(
        receive_queue.clone(),
        logger,
      ),
      uuid: UUID::new(),
      ..ComInterfaceSocket::default()
    }
  }
}

impl Default for ComInterfaceSocket {
  fn default() -> Self {
    let receive_queue = Arc::new(Mutex::new(VecDeque::new()));
    ComInterfaceSocket {
      endpoint: None,
      is_connected: false,
      is_open: false,
      is_destroyed: false,
      uuid: UUID::default(),
      logger: None,
      connection_timestamp: 0,
      receive_queue: receive_queue.clone(),
      send_queue: VecDeque::new(),
      block_collector: BlockCollector::new(receive_queue.clone()),
    }
  }
}
