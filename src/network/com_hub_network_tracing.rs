use std::sync::{Arc, Mutex};
use crate::datex_values::Endpoint;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::{
    BlockHeader, BlockType, FlagsAndTimestamp,
};
use crate::network::com_hub::ComHub;
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use crate::network::block_handler::{OutgoingScopeId, ResponseBlocks};
use crate::network::com_interfaces::com_interface_properties::InterfaceProperties;

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkTraceHopSocket {
    pub interface_type: String,
    pub interface_name: Option<String>,
    pub channel: String,
    pub socket_uuid: String,
}

impl NetworkTraceHopSocket {
    pub fn new(
        com_interface_properties: &InterfaceProperties,
        socket_uuid: ComInterfaceSocketUUID
    ) -> Self {
        NetworkTraceHopSocket {
            interface_type: com_interface_properties.interface_type.clone(),
            interface_name: com_interface_properties.name.clone(),
            channel: com_interface_properties.channel.clone(),
            socket_uuid: socket_uuid.0.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkTraceHopDirection {
    Outgoing,
    Incoming,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkTraceHop {
    #[serde_as(as = "DisplayFromStr")]
    pub endpoint: Endpoint,
    pub socket: NetworkTraceHopSocket,
    pub direction: NetworkTraceHopDirection,
}

#[derive(Debug)]
pub struct NetworkTraceResult {
    pub endpoint: Endpoint,
    pub hops: Vec<NetworkTraceHop>,
}

impl ComHub {
    pub async fn record_trace(
        self_rc: Arc<Mutex<Self>>,
        endpoint: impl Into<Endpoint>,
    ) -> Option<NetworkTraceResult> {
        let endpoint = endpoint.into();

        let trace_block = {
            let self_ref = self_rc.lock().unwrap();
            let scope_id = self_ref.block_handler.borrow_mut().get_new_scope_id().clone();
            let mut trace_block = self_ref.create_trace_block(
                vec![],
                endpoint.clone(),
                BlockType::Trace,
                scope_id,
            );
            trace_block.set_receivers(&[endpoint.clone()]);
            trace_block
        };

        let response = ComHub::send_own_block_await_response(self_rc.clone(), trace_block).await;

        assert!(response.is_ok());
        if let Ok(response) = response {
            match response {
                ResponseBlocks::SingleBlock(block) => {
                    let hops = self_rc.lock().unwrap().get_trace_data_from_block(&block)?;
                    Some(NetworkTraceResult {
                        endpoint: endpoint.clone(),
                        hops,
                    })
                }
                _ => {
                    error!("Expected single block, but got block stream");
                    None
                }
            }
        }
        else {
            error!("Failed to receive trace back block");
            None
        }
    }

    /// Handles a trace block received from another endpoint that
    /// is addressed to this endpoint.
    /// A new trace block is created and sent back to the sender.
    pub(crate) fn handle_trace_block(
        &mut self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) -> Option<()> {

        let sender = block.routing_header.sender.clone();
        info!("Received trace block from {sender}");

        // get hops vector
        let mut hops = self.get_trace_data_from_block(&block)?;

        // add incoming socket hop
        hops.push(NetworkTraceHop {
            endpoint: self.endpoint.clone(),
            socket: NetworkTraceHopSocket::new(
                self.get_com_interface_from_socket_uuid(&original_socket).borrow_mut().get_properties(),
                original_socket.clone()),
            direction: NetworkTraceHopDirection::Incoming,
        });

        // create trace back block
        let trace_back_block = self.create_trace_block(
            hops,
            sender.clone(),
            BlockType::TraceBack,
            block.block_header.scope_id.clone(),
        );

        // send trace back block
        self.send_block(trace_back_block, None);

        Some(())
    }
    
    pub(crate) fn redirect_trace_block(
        &mut self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) -> Option<()> {
        let mut block = block.clone();
        let sender = block.routing_header.sender.clone();
        info!("Redirecting trace block from {sender}");

        // add incoming socket hop
        self.add_hop_to_block_trace_data(
            &mut block,
            NetworkTraceHop {
                endpoint: self.endpoint.clone(),
                socket: NetworkTraceHopSocket::new(
                    self.get_com_interface_from_socket_uuid(&original_socket).borrow_mut().get_properties(),
                    original_socket.clone()),
                direction: NetworkTraceHopDirection::Incoming,
            },
        );

        // resend trace block
        self.send_block(block.clone(), Some(&original_socket));

        Some(())
    }

    fn create_trace_block(
        &self,
        hops: Vec<NetworkTraceHop>,
        receiver_endpoint: Endpoint,
        block_type: BlockType,
        scope_id: OutgoingScopeId,
    ) -> DXBBlock {
        let mut trace_block = DXBBlock {
            block_header: BlockHeader {
                flags_and_timestamp: FlagsAndTimestamp::default()
                    .with_block_type(block_type),
                scope_id,
                ..BlockHeader::default()
            },
            ..DXBBlock::default()
        };
        self.set_trace_data_of_block(&mut trace_block, hops);
        trace_block.set_receivers(&[receiver_endpoint.clone()]);

        trace_block
    }

    fn get_trace_data_from_block(
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

    pub(crate) fn add_hop_to_block_trace_data(
        &self,
        block: &mut DXBBlock,
        hop: NetworkTraceHop,
    ) {
        // get hops from block
        let mut hops = self.get_trace_data_from_block(block).unwrap_or_default();
        // add hop to hops
        hops.push(hop);
        // set hops to block
        self.set_trace_data_of_block(block, hops);
    }
}
