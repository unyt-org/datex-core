use crate::datex_values::Endpoint;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::{BlockHeader, BlockType, FlagsAndTimestamp};
use crate::network::com_hub::ComHub;

pub struct NetworkTrace {
    pub endpoint: Endpoint,
    pub hops_outgoing: Vec<Endpoint>,
    pub hops_incoming: Vec<Endpoint>,
}

impl ComHub {
    pub fn create_network_trace(
        &self,
        endpoint: impl Into<Endpoint>
    ) -> Option<NetworkTrace> {
        let endpoint = endpoint.into();

        let mut trace_block = DXBBlock {
            block_header: BlockHeader {
                flags_and_timestamp: FlagsAndTimestamp::default()
                    .with_block_type(
                        BlockType::Trace
                    ),
                ..BlockHeader::default()
            },
            ..DXBBlock::default()
        };

        trace_block.set_receivers(&[endpoint.clone()]);

        self.send_own_block(trace_block);

        Some(NetworkTrace {
            endpoint: endpoint.clone(),
            hops_outgoing: vec![],
            hops_incoming: vec![]
        })
    }
}