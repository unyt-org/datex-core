use crate::compile;
use crate::compiler::bytecode::compile_template;
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_values::endpoint::Endpoint;
use crate::datex_values::core_values::object::Object;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::ValueContainer;
use crate::decompiler::{decompile_body, DecompileOptions};
use crate::global::dxb_block::{DXBBlock, IncomingSection, OutgoingContextId};
use crate::global::protocol_structures::block_header::{
    BlockHeader, BlockType, FlagsAndTimestamp,
};
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::network::com_hub::{ComHub, Response, ResponseOptions};
use crate::network::com_interfaces::com_interface_properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::runtime::execution::{execute_dxb, ExecutionOptions};
use itertools::Itertools;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkTraceHopSocket {
    pub interface_type: String,
    pub interface_name: Option<String>,
    pub channel: String,
    pub socket_uuid: String,
}

impl NetworkTraceHopSocket {
    pub fn new(
        com_interface_properties: &InterfaceProperties,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Self {
        NetworkTraceHopSocket {
            interface_type: com_interface_properties.interface_type.clone(),
            interface_name: com_interface_properties.name.clone(),
            channel: com_interface_properties.channel.clone(),
            socket_uuid: socket_uuid.0.to_string(),
        }
    }
}

#[derive(
    Serialize, Deserialize, Debug, PartialEq, Clone, strum_macros::Display,
)]
pub enum NetworkTraceHopDirection {
    Outgoing,
    Incoming,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkTraceHop {
    #[serde_as(as = "DisplayFromStr")]
    pub endpoint: Endpoint,
    pub distance: i8,
    pub socket: NetworkTraceHopSocket,
    pub direction: NetworkTraceHopDirection,
    pub fork_nr: String,
    pub bounce_back: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkTraceResult {
    pub sender: Endpoint,
    pub receiver: Endpoint,
    pub hops: Vec<NetworkTraceHop>,
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
    pub fn matches_hops(&self, hops: &[(Endpoint, &str)]) -> bool {
        if self.hops.len() != hops.len() {
            return false;
        }

        for (hop, expected_hop) in self.hops.iter().zip(hops) {
            if hop.endpoint != expected_hop.0 {
                return false;
            }
            if hop.socket.interface_type != expected_hop.1 {
                return false;
            }
        }

        true
    }
}

impl Display for NetworkTraceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(
            f,
            "─────────────────────────────────────────────────────────"
        )?;
        writeln!(f, "Network trace ({} ──▶ {})", self.sender, self.receiver)?;
        writeln!(f, "  Round trip time: {:?}", self.round_trip_time)?;
        writeln!(f, "  Outbound path:")?;
        let mut hop = 1;
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

            write!(f, "    #{} via {}: ", hop, hop_1.socket.channel)?;
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
                hop = 1;
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
    pub max_hops: Option<usize>,
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
        max_hops: Option<usize>,
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
        let start_time = std::time::Instant::now();

        let responses = self
            .send_own_block_await_response(
                trace_block,
                options.response_options,
            )
            .await;
        let round_trip_time = start_time.elapsed();

        let mut results = vec![];

        for response in responses {
            match response {
                Ok(Response::ExactResponse(
                    sender,
                    IncomingSection::SingleBlock(block),
                ))
                | Ok(Response::ResolvedResponse(
                    sender,
                    IncomingSection::SingleBlock(block),
                )) => {
                    info!(
                        "Received trace block response from {}",
                        sender.clone()
                    );
                    let hops = self.get_trace_data_from_block(&block);
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
    pub(crate) fn handle_trace_block(
        &self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) -> Option<()> {
        let sender = block.routing_header.sender.clone();
        info!("Received trace block from {sender}");

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
                self.get_com_interface_from_socket_uuid(&original_socket)
                    .borrow_mut()
                    .get_properties(),
                original_socket.clone(),
            ),
            direction: NetworkTraceHopDirection::Incoming,
            fork_nr,
            bounce_back,
        });

        // create trace back block
        let trace_back_block = self.create_trace_block(
            hops,
            &[sender.clone()],
            BlockType::TraceBack,
            block.block_header.context_id,
            None,
        );

        // send trace back block
        self.send_own_block(trace_back_block);

        Some(())
    }

    pub(crate) fn handle_trace_back_block(
        &self,
        block: &DXBBlock,
        original_socket: ComInterfaceSocketUUID,
    ) -> Option<()> {
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
                    self.get_com_interface_from_socket_uuid(&original_socket)
                        .borrow_mut()
                        .get_properties(),
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

    pub(crate) fn redirect_trace_block(
        &self,
        block: DXBBlock,
        original_socket: ComInterfaceSocketUUID,
        forked: bool,
    ) -> Option<()> {
        let mut block = block.clone();
        let sender = block.routing_header.sender.clone();
        info!("Redirecting trace block from {sender}");

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
                    self.get_com_interface_from_socket_uuid(&original_socket)
                        .borrow_mut()
                        .get_properties(),
                    original_socket.clone(),
                ),
                direction: NetworkTraceHopDirection::Incoming,
                fork_nr,
                bounce_back,
            },
        );

        // resend trace block
        self.redirect_block(block, original_socket, forked);

        Some(())
    }

    fn create_trace_block(
        &self,
        hops: Vec<NetworkTraceHop>,
        receiver_endpoint: &[Endpoint],
        block_type: BlockType,
        context_id: OutgoingContextId,
        max_hops: Option<usize>,
    ) -> DXBBlock {
        let mut trace_block = DXBBlock {
            routing_header: RoutingHeader {
                ttl: max_hops.unwrap_or(42) as u8,
                ..RoutingHeader::default()
            },
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
        let hops_datex = execute_dxb(dxb, ExecutionOptions::default())
            .unwrap()
            .unwrap();
        info!("hops datex {}", hops_datex);
        if let ValueContainer::Value(Value {
            inner: CoreValue::Array(array),
            ..
        }) = hops_datex
        {
            let mut hops: Vec<NetworkTraceHop> = vec![];
            for value in array {
                if let ValueContainer::Value(Value {
                    inner: CoreValue::Object(obj),
                    ..
                }) = value
                {
                    let endpoint = obj
                        .get("endpoint")
                        .unwrap()
                        .cast_to_endpoint()
                        .unwrap();
                    let distance = obj
                        .get("distance")
                        .unwrap()
                        .cast_to_integer()
                        .unwrap()
                        .as_i128()? as i8;
                    let socket = obj.get("socket").unwrap();
                    let (interface_type, interface_name, channel, socket_uuid) =
                        if let ValueContainer::Value(Value {
                            inner: CoreValue::Object(socket_obj),
                            ..
                        }) = socket
                        {
                            let interface_type = socket_obj
                                .get("interface_type")
                                .unwrap()
                                .cast_to_text()
                                .0;
                            let interface_name =
                                if let ValueContainer::Value(Value {
                                    inner: CoreValue::Text(name),
                                    ..
                                }) = socket_obj.get("interface_name")?
                                {
                                    Some(name.clone().0)
                                } else {
                                    None
                                };
                            let channel = socket_obj
                                .get("channel")
                                .unwrap()
                                .cast_to_text()
                                .0;
                            let socket_uuid = socket_obj
                                .get("socket_uuid")
                                .unwrap()
                                .cast_to_text()
                                .0;
                            (
                                interface_type,
                                interface_name,
                                channel,
                                socket_uuid,
                            )
                        } else {
                            error!("Invalid socket data in trace block");
                            continue;
                        };
                    let direction =
                        obj.get("direction").unwrap().cast_to_text().0;
                    let fork_nr = obj.get("fork_nr").unwrap().cast_to_text().0;
                    let bounce_back = obj
                        .get("bounce_back")
                        .unwrap()
                        .cast_to_bool()
                        .unwrap()
                        .0;

                    hops.push(NetworkTraceHop {
                        endpoint,
                        distance,
                        socket: NetworkTraceHopSocket {
                            interface_type,
                            interface_name,
                            channel,
                            socket_uuid,
                        },
                        direction: match direction.as_str() {
                            "Outgoing" => NetworkTraceHopDirection::Outgoing,
                            "Incoming" => NetworkTraceHopDirection::Incoming,
                            _ => unreachable!(),
                        },
                        fork_nr,
                        bounce_back,
                    });
                }
            }
            info!("Parsed hops from trace block: {:?}", hops);
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
        let mut hops_datex = Vec::<ValueContainer>::new();

        for hop in hops {
            let mut data_obj = Object::default();

            data_obj.set("endpoint", hop.endpoint);
            data_obj.set("distance", hop.distance);

            let mut socket_obj = Object::default();
            socket_obj.set("interface_type", hop.socket.interface_type);
            socket_obj.set("interface_name", hop.socket.interface_name);
            socket_obj.set("channel", hop.socket.channel);
            socket_obj.set("socket_uuid", hop.socket.socket_uuid);

            data_obj.set("socket", ValueContainer::from(socket_obj));
            data_obj.set("direction", hop.direction.to_string());
            data_obj.set("fork_nr", hop.fork_nr);
            data_obj.set("bounce_back", hop.bounce_back);
            hops_datex.push(ValueContainer::from(data_obj));
        }

        let dxb = compile!("?", hops_datex).unwrap();
        info!(
            "Trace data: {}",
            decompile_body(&dxb, DecompileOptions::default()).unwrap()
        );

        block.body = dxb;
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
