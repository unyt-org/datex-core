use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{datex_values::Endpoint, global::dxb_block::DXBBlock};

use super::block_collector::BlockCollector;

pub struct ComInterfaceSocket {
    endpoint: Option<Endpoint>,
    is_connected: bool,
    is_open: bool,
    is_destroyed: bool,
    uuid: String,
    connection_timestamp: u64,
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
    block_collector: BlockCollector,
}

impl ComInterfaceSocket {
    pub fn new() -> ComInterfaceSocket {
        let receive_queue = Arc::new(Mutex::new(VecDeque::new()));
        ComInterfaceSocket {
            endpoint: None,
            is_connected: false,
            is_open: false,
            is_destroyed: false,
            uuid: "xyz-todo".to_string(),
            connection_timestamp: 0,
            receive_queue: receive_queue.clone(),
            block_collector: BlockCollector::new(receive_queue.clone()),
        }
    }

    pub fn get_receive_queue(&self) -> Arc<Mutex<VecDeque<u8>>> {
        self.receive_queue.clone()
    }

    pub fn get_block_queue(&self) -> &VecDeque<DXBBlock> {
        self.block_collector.get_block_queue()
    }

    pub fn update(&mut self) {
        self.block_collector.update();
    }
}
