use crate::datex_values::Endpoint;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::{
    BlockHeader, BlockType, FlagsAndTimestamp,
};
use crate::network::com_hub::ComHub;
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use log::info;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[derive(Serialize, Deserialize)]
pub struct NetworkTraceHopSocket {
    pub interface_type: String,
    pub interface_name: Option<String>,
    pub channel: String,
    pub socket_uuid: String,
}

#[derive(Serialize, Deserialize)]
pub enum NetworkTraceHopDirection {
    Outgoing,
    Incoming,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct NetworkTraceHop {
    #[serde_as(as = "DisplayFromStr")]
    pub endpoint: Endpoint,
    pub socket: NetworkTraceHopSocket,
    pub direction: NetworkTraceHopDirection,
}

pub struct NetworkTraceResult {
    pub endpoint: Endpoint,
    pub hops_outgoing: Vec<NetworkTraceHop>,
    pub hops_incoming: Vec<NetworkTraceHop>,
}

impl ComHub {
    pub fn record_trace(
        &self,
        endpoint: impl Into<Endpoint>,
    ) -> Option<NetworkTraceResult> {
        let endpoint = endpoint.into();

        let hops: Vec<NetworkTraceHop> = vec![];

        let mut trace_block = DXBBlock {
            block_header: BlockHeader {
                flags_and_timestamp: FlagsAndTimestamp::default()
                    .with_block_type(BlockType::Trace),
                ..BlockHeader::default()
            },
            ..DXBBlock::default()
        };

        trace_block.set_receivers(&[endpoint.clone()]);

        self.send_own_block(trace_block);

        Some(NetworkTraceResult {
            endpoint: endpoint.clone(),
            hops_outgoing: vec![],
            hops_incoming: vec![],
        })
    }

    pub(crate) fn handle_trace_block(
        &mut self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) -> Option<()> {
        let sender = block.routing_header.sender.clone();
        let com_interface_properties =
            self.get_com_interface_from_socket_uuid(&original_socket);
        let mut com_interface_properties =
            com_interface_properties.borrow_mut();
        let com_interface_properties =
            com_interface_properties.get_properties();

        info!("Received trace block from {sender}");
        // get hops vector
        let mut hops = self.get_trace_data_from_block(block)?;
        // add incoming socket
        hops.push(NetworkTraceHop {
            endpoint: self.endpoint.clone(),
            socket: NetworkTraceHopSocket {
                interface_type: com_interface_properties.interface_type.clone(),
                interface_name: com_interface_properties.name.clone(),
                channel: com_interface_properties.channel.clone(),
                socket_uuid: original_socket.0.to_string(),
            },
            direction: NetworkTraceHopDirection::Incoming,
        });

        // add outgoing socket
        //self.send_block()

        Some(())
    }

    fn create_trace_block(
        &self,
        hops: Vec<NetworkTraceHop>,
        receiver_endpoint: Endpoint,
        block_type: BlockType,
    ) {
        let mut trace_block = DXBBlock {
            block_header: BlockHeader {
                flags_and_timestamp: FlagsAndTimestamp::default()
                    .with_block_type(block_type),
                ..BlockHeader::default()
            },
            ..DXBBlock::default()
        };

        self.set_trace_data_of_block(&mut trace_block, hops);

        trace_block.set_receivers(&[receiver_endpoint.clone()]);
    }

    pub(crate) fn handle_trace_back_block(
        &mut self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) {
        let sender = block.routing_header.sender.clone();
        info!("Received trace back block from {sender}");
    }

    pub(crate) fn redirect_trace_block(
        &mut self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) {
        let sender = block.routing_header.sender.clone();
        info!("Redirecting trace block from {sender}");
    }

    pub(crate) fn get_trace_data_from_block(
        &self,
        block: &DXBBlock,
    ) -> Option<Vec<NetworkTraceHop>> {
        // convert json to hops
        let hops_json = String::from_utf8(block.body.clone()).ok()?;
        serde_json::from_str(&hops_json).ok()?
    }

    pub(crate) fn set_trace_data_of_block(
        &self,
        block: &mut DXBBlock,
        hops: Vec<NetworkTraceHop>,
    ) {
        // convert hops to json
        let hops_json = serde_json::to_string(&hops).unwrap();
        block.body = hops_json.into_bytes();
    }
}
