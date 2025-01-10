use crate::datex_values::Endpoint;

use super::{
    com_interface::ComInterface,
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
};

pub struct ComInterfaceSocket {
    endpoint: Option<Endpoint>,
    is_connected: bool,
    is_open: bool,
    is_destroyed: bool,
    uuid: String,
    connection_timestamp: u64,
    interface: Box<dyn ComInterface>,
}

impl ComInterfaceSocket {
    pub fn new(interface: Box<dyn ComInterface>) -> ComInterfaceSocket {
        ComInterfaceSocket {
            endpoint: None,
            is_connected: false,
            is_open: false,
            is_destroyed: false,
            uuid: "xyz-todo".to_string(),
            connection_timestamp: 0,
            interface,
        }
    }

    pub fn send_block(&mut self, block: &[u8]) -> () {
        self.interface.send_block(block);
    }

    pub fn get_channel_factor(&self) -> u32 {
        let properties = self.interface.get_properties();
        return properties.bandwidth / properties.latency;
    }

    pub fn can_send(&self) -> bool {
        let properties = self.interface.get_properties();
        return properties.direction == InterfaceDirection::OUT
            || properties.direction == InterfaceDirection::IN_OUT;
    }

    pub fn can_receive(&self) -> bool {
        let properties = self.interface.get_properties();
        return properties.direction == InterfaceDirection::IN
            || properties.direction == InterfaceDirection::IN_OUT;
    }
}
