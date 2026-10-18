use core::{fmt::Display, time::Duration};

use crate::{
    core_compiler::core_compilation_context::{
        CompileInput, DXBWithSharedValues,
    },
    global::{
        dxb_block::{DXBBlock, IncomingSection, OutgoingContextId},
        protocol_structures::{
            block_header::{BlockHeader, BlockType, FlagsAndTimestamp},
            routing_header::RoutingHeader,
        },
    },
    network::{
        com_hub::{
            ComHub,
            network_response::{Response, ResponseOptions},
        },
        com_interfaces::com_interface::{
            properties::ComInterfaceProperties, socket::ComInterfaceSocketUUID,
        },
    },
    runtime::{
        execution::{ExecutionInput, ExecutionOptions, execute_dxb_sync},
        pointer_availability_lookup::PointerAvailabilityLookup,
    },
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        core_values::endpoint::Endpoint, value::Value,
        value_container::ValueContainer,
    },
};

use crate::{
    core_compiler::value_compiler::compile_value,
    prelude::*,
    runtime::{Runtime, execution::execution_input::ExecutionCallerMetadata},
    traits::convert_value_container::ConvertValueContainer,
};
use datex_macros_internal::Datex;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_with::{DurationMilliSeconds, serde_as};
use crate::values::core_values::list::List;

#[derive(Datex, Serialize, Deserialize, Debug, Clone)]
#[datex(structural)]
pub struct NetworkTraceHopSocket {
    pub interface_type: String,
    pub interface_name: Option<String>,
    pub channel: String,
    pub socket_uuid: String,
}

impl NetworkTraceHopSocket {
    pub fn new(
        com_interface_properties: &ComInterfaceProperties,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Self {
        NetworkTraceHopSocket {
            interface_type: com_interface_properties.interface_type.clone(),
            interface_name: com_interface_properties.name.clone(),
            channel: com_interface_properties.channel.clone(),
            socket_uuid: socket_uuid.uuid_string(),
        }
    }
}

#[derive(
    Datex,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Clone,
    strum_macros::Display,
)]
#[datex(structural)]
pub enum NetworkTraceHopDirection {
    Outgoing,
    Incoming,
}

#[serde_as]
#[derive(Datex, Serialize, Deserialize, Debug, Clone)]
#[datex(structural)]
pub struct NetworkTraceHop {
    pub endpoint: Endpoint,
    pub distance: i8,
    pub socket: NetworkTraceHopSocket,
    pub direction: NetworkTraceHopDirection,
    pub fork_nr: String,
    pub bounce_back: bool,
}

#[serde_as]
#[derive(Datex, Debug, Clone, Deserialize, Serialize)]
#[datex(structural)]
pub struct NetworkTraceResult {
    pub sender: Endpoint,
    pub receiver: Endpoint,
    pub hops: Vec<NetworkTraceHop>,
    #[serde_as(as = "DurationMilliSeconds")]
    #[datex(serde)]
    pub round_trip_time: Duration,
}

impl Default for NetworkTraceResult {
    fn default() -> Self {
        NetworkTraceResult {
            sender: Endpoint::default(),
            receiver: Endpoint::ANY,
            hops: vec![],
            round_trip_time: Duration::ZERO,
        }
    }
}
impl NetworkTraceResult {
    pub(crate) fn from_hops(hops: Vec<NetworkTraceHop>) -> Self {
        let sender = hops
            .first()
            .map(|hop| hop.endpoint.clone())
            .unwrap_or_default();
        NetworkTraceResult {
            sender,
            hops,
            ..Default::default()
        }
    }
}

impl NetworkTraceResult {
    /// Checks if the hops in the network trace result match the given hops.
    /// A hop consists of an endpoint and an interface type.
    pub fn assert_matches_hops(&self, hops: &[(Endpoint, &str)]) {
        if self.hops.len() != hops.len() {
            panic!(
                "Hop count mismatch: expected {}, got {}",
                hops.len(),
                self.hops.len()
            );
        }

        for (hop, expected_hop) in self.hops.iter().zip(hops) {
            if hop.endpoint != expected_hop.0 {
                panic!(
                    "Hop endpoint mismatch: expected {}, got {}",
                    expected_hop.0, hop.endpoint
                );
            }
            if hop.socket.interface_type != expected_hop.1 {
                panic!(
                    "Hop interface type mismatch: expected {}, got {}",
                    expected_hop.1, hop.socket.interface_type
                );
            }
        }
    }
}

impl Display for NetworkTraceResult {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        writeln!(
            f,
            "─────────────────────────────────────────────────────────"
        )?;
        writeln!(f, "Network trace ({} ──▶ {})", self.sender, self.receiver)?;
        writeln!(f, "  Round trip time: {:?}", self.round_trip_time)?;
        writeln!(f, "  Outbound path:")?;
        let mut hop = 0;
        let mut is_return_path = false;
        let mut receiver_distance = 0;
        for hops in self.hops.chunks(2) {
            // missing hops
            if hops.len() < 2 {
                writeln!(f, "  Missing hops")?;
                break;
            }
            // invalid hops (1 not outgoing or 2 not incoming)
            if hops[0].direction != NetworkTraceHopDirection::Outgoing
                || hops[1].direction != NetworkTraceHopDirection::Incoming
            {
                writeln!(f, "  Invalid hops")?;
                break;
            }

            let hop_1 = &hops[0];
            let hop_2 = &hops[1];

            let distance_from_sender = if is_return_path {
                receiver_distance - hop_2.distance
            } else {
                hop_2.distance
            };

            core::write!(f, "    #{} via {}: ", hop, hop_1.socket.channel)?;
            writeln!(
                f,
                "{} ({}) ─{}▶ {} ({}) | distance from {}: {} | fork #{}",
                hop_1.endpoint,
                hop_1
                    .socket
                    .interface_name
                    .clone()
                    .unwrap_or(hop_1.socket.interface_type.clone()),
                if hop_1.bounce_back { "/" } else { "─" },
                hop_2.endpoint,
                hop_2
                    .socket
                    .interface_name
                    .clone()
                    .unwrap_or(hop_2.socket.interface_type.clone()),
                self.sender,
                distance_from_sender,
                hop_1.fork_nr,
            )?;

            // increment hop number
            hop += 1;
            // add return trip label if hop_2 endpoint is the receiver
            if !is_return_path && hop_2.endpoint == self.receiver {
                writeln!(f, "  Return path:")?;
                is_return_path = true;
                receiver_distance = hop_2.distance;
                hop = 0;
            }
        }
        writeln!(
            f,
            "─────────────────────────────────────────────────────────"
        )?;
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct TraceOptions {
    pub max_hops: Option<u8>,
    pub endpoints: Vec<Endpoint>,
    pub response_options: ResponseOptions,
}

impl TraceOptions {
    fn new_with_endpoints(endpoints: Vec<Endpoint>) -> Self {
        TraceOptions {
            endpoints,
            ..Default::default()
        }
    }

    pub fn new(
        max_hops: Option<u8>,
        response_options: ResponseOptions,
    ) -> Self {
        TraceOptions {
            max_hops,
            endpoints: vec![],
            response_options,
        }
    }
}

impl ComHub {
    pub async fn record_trace(
        &self,
        endpoint: impl Into<Endpoint>,
    ) -> Option<NetworkTraceResult> {
        self.record_trace_multiple(vec![endpoint.into()])
            .await
            .pop()
    }

    pub async fn record_trace_with_options(
        &self,
        options: TraceOptions,
    ) -> Option<NetworkTraceResult> {
        self.record_trace_multiple_with_options(options).await.pop()
    }

    pub async fn record_trace_multiple(
        &self,
        endpoints: Vec<impl Into<Endpoint>>,
    ) -> Vec<NetworkTraceResult> {
        self.record_trace_multiple_with_options(
            TraceOptions::new_with_endpoints(
                endpoints
                    .into_iter()
                    .map(|endpoint| endpoint.into())
                    .collect::<Vec<Endpoint>>(),
            ),
        )
        .await
    }

    pub async fn record_trace_multiple_with_options(
        &self,
        options: TraceOptions,
    ) -> Vec<NetworkTraceResult> {
        let endpoints =
            options.endpoints.into_iter().collect::<Vec<Endpoint>>();

        let trace_block = {
            let context_id = self.block_handler.get_new_context_id();

            self.create_trace_block(
                vec![],
                &endpoints,
                BlockType::Trace,
                context_id,
                options.max_hops,
            )
        };

        // measure round trip time
        let start_time = crate::time::now_ms();

        let responses = self
            .send_own_block_await_response(
                trace_block,
                options.response_options,
            )
            .await;
        let end_time = crate::time::now_ms();
        let round_trip_time = Duration::from_millis(end_time - start_time);

        let mut results = vec![];

        for response in responses {
            match response {
                Ok(Response::ExactResponse(
                    sender,
                    IncomingSection::SingleBlock((block, ..)),
                ))
                | Ok(Response::ResolvedResponse(
                    sender,
                    IncomingSection::SingleBlock((block, ..)),
                )) => {
                    info!(
                        "Received trace block response from {}",
                        sender.clone()
                    );
                    let block = block
                        .as_ref()
                        .expect("Expected a block in incoming section");

                    let hops = self.get_trace_data_from_block(block);
                    if let Some(hops) = hops {
                        let result = NetworkTraceResult {
                            sender: self.endpoint.clone(),
                            receiver: sender.clone(),
                            hops,
                            round_trip_time,
                        };
                        results.push(result);
                    } else {
                        error!("Failed to get trace data from block");
                        continue;
                    }
                }

                Ok(Response::UnspecifiedResponse(
                    IncomingSection::SingleBlock(_),
                )) => {
                    error!("Failed to get trace data from block");
                }
                Ok(Response::ExactResponse(
                    _,
                    IncomingSection::BlockStream(_),
                ))
                | Ok(Response::ResolvedResponse(
                    _,
                    IncomingSection::BlockStream(_),
                ))
                | Ok(Response::UnspecifiedResponse(
                    IncomingSection::BlockStream(_),
                )) => {
                    error!("Expected single block, but got block stream");
                    continue;
                }
                Err(e) => {
                    error!("Failed to receive trace block: {e}");
                }
            }
        }

        results
    }

    /// Handles a trace block received from another endpoint that
    /// is addressed to this endpoint.
    /// A new trace block is created and sent back to the sender.
    pub(crate) async fn handle_trace_block(
        &self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) -> Option<()> {
        let sender = block.routing_header.sender.clone();
        info!("Received trace block from {sender}");
        let socket_data =
            self.socket_manager.get_socket_by_uuid(&original_socket)?;

        // get hops vector
        let mut hops = self.get_trace_data_from_block(block)?;

        // fork_nr stays the same
        let fork_nr = self.get_current_fork_from_trace_block(block);
        let bounce_back = block.is_bounce_back();

        // add incoming socket hop
        hops.push(NetworkTraceHop {
            endpoint: self.endpoint.clone(),
            distance: block.routing_header.distance,
            socket: NetworkTraceHopSocket::new(
                &socket_data.interface_properties,
                original_socket.clone(),
            ),
            direction: NetworkTraceHopDirection::Incoming,
            fork_nr,
            bounce_back,
        });

        // create trace back block
        let trace_back_block = self.create_trace_block(
            hops,
            core::slice::from_ref(&sender),
            BlockType::TraceBack,
            block.block_header.context_id,
            None,
        );

        // send trace back block
        // TODO #380: handle error and resend error and stuff
        let _ = self.send_own_block_async(trace_back_block).await;

        Some(())
    }

    pub(crate) fn handle_trace_back_block(
        &self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) -> Option<()> {
        let socket_data =
            self.socket_manager.get_socket_by_uuid(&original_socket)?;

        let mut block = block.clone();
        let sender = block.routing_header.sender.clone();
        info!("Received trace back block from {sender}");

        let distance = block.routing_header.distance;

        // fork_nr stays the same
        let fork_nr = self.get_current_fork_from_trace_block(&block);
        let bounce_back = block.is_bounce_back();

        self.add_hop_to_block_trace_data(
            &mut block,
            NetworkTraceHop {
                endpoint: self.endpoint.clone(),
                distance,
                socket: NetworkTraceHopSocket::new(
                    &socket_data.interface_properties,
                    original_socket.clone(),
                ),
                direction: NetworkTraceHopDirection::Incoming,
                fork_nr,
                bounce_back,
            },
        );

        // send network trace result to the receiver
        self.block_handler.handle_incoming_block(block);
        Some(())
    }

    pub(crate) async fn redirect_trace_block(
        &self,
        block: DXBBlock,
        original_socket: ComInterfaceSocketUUID,
        forked: bool,
    ) -> Option<()> {
        let mut block = block.clone();
        let sender = block.routing_header.sender.clone();
        info!("Redirecting trace block from {sender}");
        let socket_data =
            self.socket_manager.get_socket_by_uuid(&original_socket)?;

        let hops = self.get_trace_data_from_block(&block).unwrap_or_default();
        info!("{}", NetworkTraceResult::from_hops(hops));

        // add incoming socket hop
        let distance = block.routing_header.distance;
        // fork_nr stays the same
        let fork_nr = self.get_current_fork_from_trace_block(&block);
        let bounce_back = block.is_bounce_back();

        self.add_hop_to_block_trace_data(
            &mut block,
            NetworkTraceHop {
                endpoint: self.endpoint.clone(),
                distance,
                socket: NetworkTraceHopSocket::new(
                    &socket_data.interface_properties,
                    original_socket.clone(),
                ),
                direction: NetworkTraceHopDirection::Incoming,
                fork_nr,
                bounce_back,
            },
        );

        // resend trace block
        let _ = self.redirect_block(block, original_socket, forked).await; // FIXME result handling

        Some(())
    }

    fn create_trace_block(
        &self,
        hops: Vec<NetworkTraceHop>,
        receiver_endpoint: &[Endpoint],
        block_type: BlockType,
        context_id: OutgoingContextId,
        max_hops: Option<u8>,
    ) -> DXBBlock {
        let mut trace_block = DXBBlock {
            routing_header: RoutingHeader::unsigned()
                .with_ttl(max_hops.unwrap_or(42))
                .to_owned(),
            block_header: BlockHeader {
                flags_and_timestamp: FlagsAndTimestamp::default()
                    .with_block_type(block_type),
                context_id,
                ..BlockHeader::default()
            },
            ..DXBBlock::default()
        };
        self.set_trace_data_of_block(&mut trace_block, hops);
        trace_block.set_receivers(receiver_endpoint);
        trace_block
    }

    pub(crate) fn get_trace_data_from_block(
        &self,
        block: &DXBBlock,
    ) -> Option<Vec<NetworkTraceHop>> {
        // convert DATEX to hops
        let dxb = block.body.clone();
        let runtime_stub = Runtime::stub(); // FIXME
        let exec_input = ExecutionInput::new(
            DXBWithSharedValues::new(dxb, vec![]),
            ExecutionCallerMetadata::local_default(),
            ExecutionOptions::default(),
            runtime_stub,
        );
        let hops_datex =
            execute_dxb_sync(exec_input).expect("Failed to execute DATEX");

        if let Some(hops_datex) = hops_datex &&
            let Ok(list) = hops_datex.try_into_value::<List>()
        {
            let hops: Vec<NetworkTraceHop> = list
                .into_iter()
                .map(NetworkTraceHop::try_convert_from_value_container)
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            Some(hops)
        } else {
            None
        }
    }

    /// get a new fork number if fork_count is greater than 0, e.g.
    /// current fork_nr = '0', fork_count = 1 -> '01'
    /// current fork_nr = '0', fork_count = 2 -> '02'
    /// current fork_nr = '1', fork_count = 0 -> '1'
    /// current fork_nr = '1', fork_count = 1 -> '11'
    pub(crate) fn calculate_fork_nr(
        &self,
        block: &DXBBlock,
        fork_count: Option<usize>,
    ) -> String {
        let current_fork_nr = self
            .get_trace_data_from_block(block)
            .unwrap_or_default()
            .last()
            .map(|hop| hop.fork_nr.clone())
            .unwrap_or_default();
        if let Some(fork_count) = fork_count {
            // append new fork number to the end of the string
            format!("{current_fork_nr}{fork_count:X}")
        } else {
            // return current fork number
            if current_fork_nr.is_empty() {
                "0".to_string()
            } else {
                current_fork_nr
            }
        }
    }

    pub(crate) fn get_current_fork_from_trace_block(
        &self,
        block: &DXBBlock,
    ) -> String {
        self.get_trace_data_from_block(block)
            .unwrap_or_default()
            .last()
            .map(|hop| hop.fork_nr.clone())
            .unwrap_or_else(|| "0".to_string())
    }

    pub(crate) fn set_trace_data_of_block(
        &self,
        block: &mut DXBBlock,
        hops: Vec<NetworkTraceHop>,
    ) {
        // convert hops to DATEX
        let hops_datex = hops
            .into_iter()
            .map(|hop| ValueContainer::Local(Value::native(hop)))
            .collect::<Vec<ValueContainer>>();

        let pointer_lookup = PointerAvailabilityLookup::default();
        let dxb = compile_value(
            Value::from(hops_datex),
            CompileInput::new(&pointer_lookup, &[]),
        );
        // info!(
        //     "Trace data: {}",
        //     decompile_body(&dxb, DecompileOptions::default()).unwrap()
        // );

        block.body = dxb.dxb;
    }

    pub(crate) fn add_hop_to_block_trace_data(
        &self,
        block: &mut DXBBlock,
        hop: NetworkTraceHop,
    ) {
        // get hops from block
        let mut hops =
            self.get_trace_data_from_block(block).unwrap_or_default();
        // add hop to hops
        hops.push(hop);
        // set hops to block
        self.set_trace_data_of_block(block, hops);
    }
}

#[cfg(all(test, feature = "std"))]
pub mod tests {
    use crate::network::com_hub::test_utils::{
        TEST_ENDPOINT_A, TEST_ENDPOINT_B, run_with_coupled_com_hubs,
    };
    use ntest_timeout::timeout;

    #[tokio::test]
    #[timeout(1000)]
    async fn create_network_trace() {
        flexi_logger::init();
        run_with_coupled_com_hubs(async move |a, _b| {
            // send trace from A to B
            let network_trace =
                a.com_hub.record_trace(TEST_ENDPOINT_B.clone()).await;
            assert!(network_trace.is_some());
            println!("Network trace:\n{}", network_trace.as_ref().unwrap());

            network_trace.unwrap().assert_matches_hops(&[
                (TEST_ENDPOINT_A.clone(), "test-interface"),
                (TEST_ENDPOINT_B.clone(), "test-interface"),
                (TEST_ENDPOINT_B.clone(), "test-interface"),
                (TEST_ENDPOINT_A.clone(), "test-interface"),
            ]);
        })
        .await;
    }
}
