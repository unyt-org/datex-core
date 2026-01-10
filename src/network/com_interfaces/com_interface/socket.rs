use core::prelude::rust_2024::*;
use strum_macros::EnumIs;

use crate::network::com_interfaces::block_collector::BlockCollector;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface::properties::InterfaceDirection;
use crate::std_sync::Mutex;
use crate::stdlib::string::String;
use crate::stdlib::sync::Arc;
use crate::stdlib::vec::Vec;
use crate::task::{
    UnboundedReceiver, UnboundedSender, create_unbounded_channel,
};
use crate::utils::once_consumer::OnceConsumer;
use crate::utils::uuid::UUID;
use crate::{
    global::dxb_block::DXBBlock, values::core_values::endpoint::Endpoint,
};
use core::fmt::Display;

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
pub enum ComInterfaceSocketEvent {
    NewSocket(ComInterfaceSocket),
    RemovedSocket(ComInterfaceSocketUUID),
    RegisteredSocket(ComInterfaceSocketUUID, i8, Endpoint),
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
    block_in_receiver: OnceConsumer<UnboundedReceiver<DXBBlock>>,
}

impl ComInterfaceSocket {
    /// Received blocks coming from the interface (to be processed by the ComHub)
    pub fn take_block_in_receiver(&mut self) -> UnboundedReceiver<DXBBlock> {
        self.block_in_receiver.consume()
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
    ) -> (ComInterfaceSocket, UnboundedSender<Vec<u8>>) {
        let (bytes_in_sender, block_in_receiver) = BlockCollector::init();
        (
            ComInterfaceSocket {
                direct_endpoint: None,
                state: SocketState::Created,
                uuid: ComInterfaceSocketUUID(UUID::new()),
                interface_uuid,
                connection_timestamp: 0,
                channel_factor,
                direction,
                block_in_receiver: OnceConsumer::new(block_in_receiver),
            },
            bytes_in_sender,
        )
    }
}
