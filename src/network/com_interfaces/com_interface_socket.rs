use core::prelude::rust_2024::*;
use strum_macros::EnumIs;

use super::block_collector::BlockCollector;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::InterfaceDirection;
use crate::std_sync::Mutex;
use crate::stdlib::string::String;
use crate::stdlib::sync::Arc;
use crate::stdlib::vec::Vec;
use crate::task::{create_unbounded_channel, UnboundedReceiver, UnboundedSender};
use crate::utils::uuid::UUID;
use crate::{
    global::dxb_block::DXBBlock, values::core_values::endpoint::Endpoint,
};
use core::fmt::Display;
use crate::utils::once_consumer::OnceConsumer;

#[derive(Debug, Clone, Copy, PartialEq, EnumIs)]
pub enum SocketState {
    Created,
    Open,
    Destroyed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComInterfaceSocketUUID(pub UUID);
impl Display for ComInterfaceSocketUUID {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::write!(f, "ComInterfaceSocket({})", self.0)
    }
}
impl ComInterfaceSocketUUID {
    pub fn from_string(s: String) -> ComInterfaceSocketUUID {
        ComInterfaceSocketUUID(UUID::from_string(s))
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
    pub bytes_in_sender: Arc<Mutex<UnboundedSender<Vec<u8>>>>,
    block_in_receiver: OnceConsumer<UnboundedReceiver<DXBBlock>>,
    
    bytes_out_sender: Arc<Mutex<UnboundedSender<Vec<u8>>>>,
    pub bytes_out_receiver: OnceConsumer<UnboundedReceiver<Vec<u8>>>,
}

impl ComInterfaceSocket {
    pub fn take_block_in_receiver(&mut self) -> UnboundedReceiver<DXBBlock> {
        self.block_in_receiver.consume()
    }
    
    pub fn take_bytes_out_receiver(&mut self) -> UnboundedReceiver<Vec<u8>> {
        self.bytes_out_receiver.consume()
    }

    pub fn queue_outgoing_block(&mut self, block: &[u8]) {
        self.bytes_out_sender
            .lock()
            .unwrap()
            .start_send(block.to_vec())
            .unwrap();
    }

    pub fn can_send(&self) -> bool {
        self.direction == InterfaceDirection::Out
            || self.direction == InterfaceDirection::InOut
    }

    pub fn can_receive(&self) -> bool {
        self.direction == InterfaceDirection::In
            || self.direction == InterfaceDirection::InOut
    }

    /// Initializes a new ComInterfaceSocket, starts the BlockCollector task.
    pub fn init(
        interface_uuid: ComInterfaceUUID,
        direction: InterfaceDirection,
        channel_factor: u32,
    ) -> ComInterfaceSocket {
        let (bytes_in_sender, block_in_receiver) = BlockCollector::init();
        let (bytes_out_sender, bytes_out_receiver) = create_unbounded_channel::<Vec<u8>>();
        ComInterfaceSocket {
            direct_endpoint: None,
            state: SocketState::Created,
            uuid: ComInterfaceSocketUUID(UUID::new()),
            interface_uuid,
            connection_timestamp: 0,
            channel_factor,
            direction,
            bytes_in_sender: Arc::new(Mutex::new(bytes_in_sender)),
            block_in_receiver: OnceConsumer::new(block_in_receiver),
            bytes_out_sender: Arc::new(Mutex::new(bytes_out_sender)),
            bytes_out_receiver: OnceConsumer::new(bytes_out_receiver),
        }
    }
}
