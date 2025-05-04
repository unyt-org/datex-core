use strum_macros::EnumIs;

use super::block_collector::BlockCollector;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::InterfaceDirection;
use crate::stdlib::fmt::Display;
use crate::stdlib::{collections::VecDeque, sync::Arc};
use crate::utils::uuid::UUID;
use crate::{datex_values::Endpoint, global::dxb_block::DXBBlock};
use std::sync::Mutex;
// FIXME no-std

#[derive(Debug, Clone, Copy, PartialEq, EnumIs)]
pub enum SocketState {
    Created,
    Open,
    Destroyed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComInterfaceSocketUUID(pub UUID);
impl Display for ComInterfaceSocketUUID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ComInterfaceSocket({})", self.0)
    }
}

#[derive(Debug)]
pub struct ComInterfaceSocket {
    pub direct_endpoint: Option<Endpoint>,
    pub state: SocketState,
    pub uuid: ComInterfaceSocketUUID,
    pub interface_uuid: ComInterfaceUUID,
    pub connection_timestamp: u64,
    pub channel_factor: u32,
    pub direction: InterfaceDirection,
    pub receive_queue: Arc<Mutex<VecDeque<u8>>>,
    pub send_queue: VecDeque<Vec<u8>>,
    pub block_collector: BlockCollector,
}

impl ComInterfaceSocket {
    pub fn get_receive_queue(&self) -> Arc<Mutex<VecDeque<u8>>> {
        self.receive_queue.clone()
    }

    pub fn get_incoming_block_queue(&mut self) -> &mut VecDeque<DXBBlock> {
        self.block_collector.get_block_queue()
    }

    pub fn collect_incoming_data(&mut self) {
        self.block_collector.update();
    }

    pub fn queue_outgoing_block(&mut self, block: &[u8]) {
        self.send_queue.push_back(block.to_vec());
    }

    pub fn can_send(&self) -> bool {
        self.direction == InterfaceDirection::Out
            || self.direction == InterfaceDirection::InOut
    }

    pub fn can_receive(&self) -> bool {
        self.direction == InterfaceDirection::In
            || self.direction == InterfaceDirection::InOut
    }

    pub fn new(
        interface_uuid: ComInterfaceUUID,
        direction: InterfaceDirection,
        channel_factor: u32,
    ) -> ComInterfaceSocket {
        let receive_queue = Arc::new(Mutex::new(VecDeque::new()));
        ComInterfaceSocket::new_with_receive_queue(
            interface_uuid,
            receive_queue,
            direction,
            channel_factor,
        )
    }

    pub fn new_with_receive_queue(
        interface_uuid: ComInterfaceUUID,
        receive_queue: Arc<Mutex<VecDeque<u8>>>,
        direction: InterfaceDirection,
        channel_factor: u32,
    ) -> ComInterfaceSocket {
        ComInterfaceSocket {
            receive_queue: receive_queue.clone(),
            block_collector: BlockCollector::new(receive_queue.clone()),
            interface_uuid,
            direct_endpoint: None,
            state: SocketState::Created,
            uuid: ComInterfaceSocketUUID(UUID::new()),
            connection_timestamp: 0,
            channel_factor,
            direction,
            send_queue: VecDeque::new(),
        }
    }
}
