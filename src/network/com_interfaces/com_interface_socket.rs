use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{datex_values::Endpoint, global::dxb_block::DXBBlock};

use super::block_collector::BlockCollector;

pub struct ComInterfaceSocket {
    pub endpoint: Option<Endpoint>,
    pub is_connected: bool,
    pub is_open: bool,
    pub is_destroyed: bool,
    pub uuid: String,
    pub connection_timestamp: u64,
    pub receive_queue: Arc<Mutex<VecDeque<u8>>>,
    pub send_queue: VecDeque<Vec<u8>>,
    pub block_collector: BlockCollector,
}

impl ComInterfaceSocket {

    pub fn get_receive_queue(&self) -> Arc<Mutex<VecDeque<u8>>> {
        self.receive_queue.clone()
    }

    pub fn get_incoming_block_queue(&self) -> &VecDeque<DXBBlock> {
        self.block_collector.get_block_queue()
    }

    pub fn collect_incoming_data(&mut self) {
        self.block_collector.update();
    }

    pub fn queue_outgoing_block(&mut self, block: &[u8]) {
        self.send_queue.push_back(block.to_vec());
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
            uuid: "xyz-todo".to_string(),
            connection_timestamp: 0,
            receive_queue: receive_queue.clone(),
            send_queue: VecDeque::new(),
            block_collector: BlockCollector::new(receive_queue.clone()),
        }
    }
}