use crate::datex_values::Endpoint;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::network::com_hub::ComHub;

pub struct NetworkTrace {
    pub endpoint: Endpoint,
    pub hops_outgoing: Vec<Endpoint>,
    pub hops_incoming: Vec<Endpoint>,
}

impl ComHub {
    pub fn create_network_trace(
        &self,
        endpoint: Endpoint
    ) -> Option<NetworkTrace> {
        
        let trace_block = DXBBlock {
            routing_header: RoutingHeader {
                ..RoutingHeader::default()
            },
            ..DXBBlock::default()
        };
        
        self.send_own_block(trace_block);
        
        todo!()
    }
}
