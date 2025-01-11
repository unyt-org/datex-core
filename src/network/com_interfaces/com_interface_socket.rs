use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::datex_values::Endpoint;

pub struct ComInterfaceSocket {
    endpoint: Option<Endpoint>,
    is_connected: bool,
    is_open: bool,
    is_destroyed: bool,
    uuid: String,
    connection_timestamp: u64,
}

impl ComInterfaceSocket {
    pub fn new() -> ComInterfaceSocket {
        ComInterfaceSocket {
            endpoint: None,
            is_connected: false,
            is_open: false,
            is_destroyed: false,
            uuid: "xyz-todo".to_string(),
            connection_timestamp: 0,
        }
    }

    pub fn get_receive_queue(&self) -> Option<Arc<Mutex<VecDeque<u8>>>> {
        None
    }
}
