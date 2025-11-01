use super::mockup_interface::{MockupInterface, store_sender_and_receiver};
use crate::network::helpers::mockup_interface::MockupInterfaceSetupData;
use core::panic;
use datex_core::network::com_hub::{ComInterfaceFactoryFn, InterfacePriority};
use datex_core::network::com_hub_network_tracing::TraceOptions;
use datex_core::network::com_interfaces::com_interface::ComInterfaceFactory;
use datex_core::network::com_interfaces::com_interface_properties::InterfaceDirection;
use datex_core::runtime::{Runtime, RuntimeConfig};
use datex_core::serde::serializer::to_value_container;
use datex_core::values::core_values::endpoint::Endpoint;
use datex_core::values::value_container::ValueContainer;
use log::info;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use core::fmt::{self, Debug, Display};
use std::path::Path;
use std::rc::Rc;
use core::str::FromStr;
use std::sync::mpsc;
use std::{env, fs};

pub struct InterfaceConnection {
    interface_type: String,
    priority: InterfacePriority,
    pub setup_data: Option<MockupInterfaceSetupData>,
    pub endpoint: Option<Endpoint>,
}

impl InterfaceConnection {
    pub fn new(
        interface_type: &str,
        priority: InterfacePriority,
        setup_data: MockupInterfaceSetupData,
    ) -> Self {
        InterfaceConnection {
            interface_type: interface_type.to_string(),
            priority,
            setup_data: Some(setup_data),
            endpoint: None,
        }
    }

    pub fn with_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoint = Some(endpoint);
        self
    }
}

pub struct Node {
    pub endpoint: Endpoint,
    pub connections: Vec<InterfaceConnection>,
    pub runtime: Option<Rc<Runtime>>,
}

impl Node {
    pub fn new(endpoint: impl Into<Endpoint>) -> Self {
        Node {
            endpoint: endpoint.into(),
            connections: Vec::new(),
            runtime: None,
        }
    }

    pub fn with_connection(mut self, connection: InterfaceConnection) -> Self {
        self.connections.push(connection);
        self
    }
}

pub struct MockupInterfaceChannelEndpoint {
    sender: mpsc::Sender<Vec<u8>>,
    receiver: mpsc::Receiver<Vec<u8>>,
}

type MockupInterfaceChannels =
    HashMap<String, Option<MockupInterfaceChannelEndpoint>>;

pub struct Network {
    pub is_initialized: bool,
    pub endpoints: Vec<Node>,
    com_interface_factories: HashMap<String, ComInterfaceFactoryFn>,
}
#[derive(Clone)]
pub struct Route {
    pub receiver: Endpoint,
    pub hops: Vec<(Endpoint, Option<String>, Option<String>)>,
    // temp remember last fork
    pub next_fork: Option<String>,
}

impl Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (endpoint, channel, _fork)) in self.hops.iter().enumerate() {
            // Write the endpoint
            core::write!(f, "{endpoint}")?;

            // If not the last, write the arrow + optional channel
            if i + 1 < self.hops.len() {
                if let Some(chan) = channel {
                    core::write!(f, " -({chan})-> ")?;
                } else {
                    core::write!(f, " --> ")?;
                }
            }
        }
        Ok(())
    }
}

impl Route {
    pub fn between<R>(source: R, receiver: R) -> Self
    where
        R: TryInto<Endpoint>,
        R::Error: Debug,
    {
        Route {
            receiver: receiver.try_into().expect("Invalid receiver endpoint"),
            hops: vec![(
                source.try_into().expect("Invalid source endpoint"),
                None,
                None,
            )],
            next_fork: None,
        }
    }

    pub fn hop<R>(mut self, target: R) -> Self
    where
        R: TryInto<Endpoint>,
        R::Error: Debug,
    {
        self.add_hop(target.try_into().expect("Invalid target endpoint"));
        self
    }

    pub fn fork(mut self, fork_nr: &str) -> Self {
        self.next_fork = Some(fork_nr.to_string());
        self
    }

    pub fn to_via<R>(mut self, target: R, channel: &str) -> Self
    where
        R: TryInto<Endpoint>,
        R::Error: Debug,
    {
        let len = self.hops.len();
        if len > 0 {
            self.hops[len - 1].1 = Some(channel.to_string());
        }
        self.add_hop(target.try_into().expect("Invalid target endpoint"));
        self
    }

    pub fn back(mut self) -> Self {
        if self.hops.len() >= 2 {
            let len = self.hops.len();
            let to = self.hops[len - 2].0.clone();
            let channel = self.hops[len - 2].1.clone();
            self.hops[len - 1].1 = Some(channel.clone().unwrap_or_default());
            self.add_hop(to);
        }
        self
    }
    pub fn back_via(mut self, channel: &str) -> Self {
        if self.hops.len() >= 2 {
            let len = self.hops.len();
            let to = self.hops[len - 2].0.clone();
            self.hops[len - 1].1 = Some(channel.to_string());
            self.add_hop(to);
        }
        self
    }

    fn add_hop(&mut self, to: impl Into<Endpoint>) {
        let fork = self.next_fork.take();
        self.hops.push((to.into(), None, fork));
    }

    /// Converts the Route into a sequence of (from, channel, to) triples
    pub fn to_segments(&self) -> Vec<(Endpoint, String, Endpoint)> {
        let mut segments = Vec::new();
        for w in self.hops.windows(2) {
            if let [(from, Some(chan), _), (to, _, _)] = &w {
                segments.push((from.clone(), chan.clone(), to.clone()));
            }
        }
        segments
    }

    pub async fn test(
        &self,
        network: &Network,
    ) -> Result<(), RouteAssertionError> {
        self.test_with_options(network, TraceOptions::default())
            .await
    }

    pub async fn test_with_options(
        &self,
        network: &Network,
        options: TraceOptions,
    ) -> Result<(), RouteAssertionError> {
        test_routes(&[self.clone()], network, options).await
    }
}

pub async fn test_routes(
    routes: &[Route],
    network: &Network,
    options: TraceOptions,
) -> Result<(), RouteAssertionError> {
    let start = routes[0].hops[0].0.clone();
    let ends = routes
        .iter()
        .map(|r| r.hops.last().unwrap().0.clone())
        .collect::<Vec<_>>();

    // make sure the start endpoint for all routes is the same
    for route in routes {
        if route.hops[0].0 != start {
            core::panic!(
                "Route start endpoints must all be the same. Found {} instead of {}",
                route.hops[0].0, start
            );
        }
    }

    for end in ends {
        if start != end {
            core::panic!("Route start {} does not match receiver {}", start, end);
        }
    }

    let network_traces = network
        .get_runtime(start)
        .com_hub()
        .record_trace_multiple_with_options(TraceOptions {
            endpoints: routes.iter().map(|r| r.receiver.clone()).collect(),
            ..options
        })
        .await;

    // combine received traces with original routes
    let route_pairs = routes
        .iter()
        .map(|route| {
            // find matching route with the same receiver in network_traces
            network_traces
                .iter()
                .find(|t| t.receiver == route.receiver)
                .ok_or_else(|| {
                    RouteAssertionError::MissingResponse(route.receiver.clone())
                })
                .map(|trace| (trace, route))
        })
        .collect::<Result<Vec<_>, _>>()?;

    for (trace, route) in route_pairs {
        // print network trace
        info!("Network trace:\n{trace}");

        let mut index = 0;

        // combine original and expected hops
        let hop_pairs = trace
            .hops
            .iter()
            .enumerate()
            .filter_map(
                |(i, h)| {
                    if i % 2 == 1 || i == 0 { Some(h) } else { None }
                },
            )
            .zip(route.hops.iter());

        for (original, (expected_endpoint, expected_channel, expected_fork)) in
            hop_pairs
        {
            // check endpoint
            if original.endpoint != expected_endpoint.clone() {
                return Err(RouteAssertionError::InvalidEndpointOnHop(
                    index,
                    expected_endpoint.clone(),
                    original.endpoint.clone(),
                ));
            }
            // check channel
            if let Some(channel) = &expected_channel {
                if original.socket.interface_name != Some(channel.clone()) {
                    return Err(RouteAssertionError::InvalidChannelOnHop(
                        index,
                        channel.clone(),
                        original
                            .socket
                            .interface_name
                            .clone()
                            .unwrap_or("None".to_string()),
                    ));
                }
            }
            // check fork
            if let Some(fork) = expected_fork {
                if &original.fork_nr != fork {
                    return Err(RouteAssertionError::InvalidForkOnHop(
                        index,
                        fork.clone(),
                        original.fork_nr.clone(),
                    ));
                }
            }
            index += 1;
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum RouteAssertionError {
    InvalidEndpointOnHop(i32, Endpoint, Endpoint),
    InvalidChannelOnHop(i32, String, String),
    InvalidForkOnHop(i32, String, String),
    MissingResponse(Endpoint),
}

impl Display for RouteAssertionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouteAssertionError::InvalidEndpointOnHop(
                index,
                expected,
                actual,
            ) => {
                core::write!(
                    f,
                    "Expected hop #{index} to be {expected} but was {actual}"
                )
            }
            RouteAssertionError::InvalidChannelOnHop(
                index,
                expected,
                actual,
            ) => {
                core::write!(
                    f,
                    "Expected hop #{index} to be channel {expected} but was {actual}"
                )
            }
            RouteAssertionError::InvalidForkOnHop(index, expected, actual) => {
                core::write!(
                    f,
                    "Expected hop #{index} to be fork {expected} but was {actual}"
                )
            }
            RouteAssertionError::MissingResponse(endpoint) => {
                core::write!(f, "No response received for endpoint {endpoint}")
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct NetworkNode {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Deserialize)]
struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub priority: i16,
    pub endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NetworkData {
    pub nodes: Vec<NetworkNode>,
    pub edges: Vec<Edge>,
}

impl Network {
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let current_dir =
            env::current_dir().expect("Failed to get current directory");
        let path = current_dir
            .join("tests/network/network-builder/networks/")
            .join(path);
        info!("Loading network from {}", path.display());

        let file_content =
            fs::read_to_string(path).expect("Failed to read the file");
        let network_data: NetworkData = serde_json::from_str(&file_content)
            .expect("Failed to deserialize the JSON");

        let mut nodes = Vec::new();
        let channel_names = network_data
            .edges
            .iter()
            .map(|edge| {
                let mut channel = [
                    edge.edge_type.clone(),
                    edge.source.clone(),
                    edge.target.clone(),
                ];
                channel.sort();
                channel.join("_")
            })
            .collect::<Vec<_>>();

        for network_node in network_data.nodes.iter() {
            let endpoint =
                Endpoint::from_str(&network_node.label.clone()).unwrap();
            let mut node = Node::new(endpoint);

            for edge in network_data.edges.iter() {
                let mut channel = [
                    edge.edge_type.clone(),
                    edge.source.clone(),
                    edge.target.clone(),
                ];
                channel.sort();
                let channel = channel.join("_");
                let is_bidirectional = channel_names
                    .iter()
                    .filter(|&item| item == &channel)
                    .count()
                    == 2;
                let is_outgoing = edge.source == network_node.id;

                if is_outgoing
                    || (edge.target == network_node.id && !is_bidirectional)
                {
                    info!(
                        "{} is_outgoing: {}, is_bidirectional: {}",
                        network_node.id, is_outgoing, is_bidirectional
                    );

                    let interface_direction = if is_bidirectional {
                        InterfaceDirection::InOut
                    } else if is_outgoing {
                        InterfaceDirection::Out
                    } else {
                        InterfaceDirection::In
                    };

                    let prio = {
                        if edge.priority >= 0
                            && interface_direction != InterfaceDirection::In
                        {
                            InterfacePriority::Priority(edge.priority as u16)
                        } else {
                            InterfacePriority::None
                        }
                    };

                    if edge.edge_type == "mockup" {
                        info!(
                            "Channel: {channel:?}, Direction: {interface_direction:?}"
                        );

                        let other_endpoint = edge
                            .endpoint
                            .as_deref()
                            .map(Endpoint::from_str)
                            .map(|e| e.unwrap());

                        if let Some(endpoint) = other_endpoint {
                            node = node.with_connection(InterfaceConnection::new(
                                &edge.edge_type,
                                prio,
                                MockupInterfaceSetupData::new_with_endpoint_and_direction(
                                    &channel,
                                    endpoint,
                                    interface_direction,
                                ),
                            ));
                        } else {
                            node = node
                                .with_connection(InterfaceConnection::new(
                                &edge.edge_type,
                                prio,
                                MockupInterfaceSetupData::new_with_direction(
                                    &channel,
                                    interface_direction,
                                ),
                            ));
                        }
                    }
                }
            }
            nodes.push(node);
        }
        let mut network = Network::create(nodes);
        network.register_interface("mockup", MockupInterface::factory);
        network
    }

    pub fn create(mut endpoints: Vec<Node>) -> Self {
        let mut mockup_interface_channels = HashMap::new();

        // iterate over all endpoints and handle mockup endpoints
        for endpoint in endpoints.iter_mut() {
            for connection in endpoint.connections.iter_mut() {
                if connection.interface_type == "mockup" {
                    Network::init_mockup_endpoint(
                        connection,
                        &mut mockup_interface_channels,
                    );
                }
            }
        }
        info!(
            "Mockup channels: {:?}",
            mockup_interface_channels
                .values()
                .map(|c| c.is_some())
                .collect::<Vec<_>>()
        );

        Network {
            is_initialized: false,
            endpoints,
            com_interface_factories: HashMap::new(),
        }
    }

    fn init_mockup_endpoint(
        connection: &mut InterfaceConnection,
        mockup_interface_channels: &mut MockupInterfaceChannels,
    ) {
        // get setup data as MockupInterfaceSetupData
        if let Some(setup_data) = &mut connection.setup_data {
            let channel = Network::get_mockup_interface_channel(
                mockup_interface_channels,
                setup_data.name.clone(),
            );
            match setup_data.direction {
                InterfaceDirection::In => {
                    setup_data.channel_index = Some(store_sender_and_receiver(
                        None,
                        Some(channel.receiver),
                    ));
                }
                InterfaceDirection::Out => {
                    setup_data.channel_index = Some(store_sender_and_receiver(
                        Some(channel.sender),
                        None,
                    ));
                }
                InterfaceDirection::InOut => {
                    setup_data.channel_index = Some(store_sender_and_receiver(
                        Some(channel.sender),
                        Some(channel.receiver),
                    ));
                }
            }

            info!("setup_data: {:?}", setup_data);
        }
    }

    fn get_mockup_interface_channel(
        mockup_interface_channels: &mut MockupInterfaceChannels,
        name: String,
    ) -> MockupInterfaceChannelEndpoint {
        if !mockup_interface_channels.contains_key(&name) {
            let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
            let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

            mockup_interface_channels.insert(
                name,
                Some(MockupInterfaceChannelEndpoint {
                    sender: sender_b,
                    receiver: receiver_a,
                }),
            );

            MockupInterfaceChannelEndpoint {
                sender: sender_a,
                receiver: receiver_b,
            }
        } else {
            match mockup_interface_channels.get_mut(&name).unwrap().take() {
                Some(channel) => channel,
                _ => {
                    core::panic!("Channel {name} is already used");
                }
            }
        }
    }

    pub fn register_interface(
        &mut self,
        interface_type: &str,
        factory: ComInterfaceFactoryFn,
    ) {
        self.com_interface_factories
            .insert(interface_type.to_string(), factory);
    }

    pub async fn start(&mut self) {
        if self.is_initialized {
            core::panic!("Network already initialized");
        }
        self.is_initialized = true;

        // create new runtimes for each endpoint
        for endpoint in self.endpoints.iter_mut() {
            let runtime = Rc::new(Runtime::new(
                RuntimeConfig::new_with_endpoint(endpoint.endpoint.clone()),
            ));

            // register factories
            for (interface_type, factory) in self.com_interface_factories.iter()
            {
                runtime.com_hub().register_interface_factory(
                    interface_type.clone(),
                    *factory,
                )
            }

            // add com interfaces
            for connection in endpoint.connections.iter_mut() {
                runtime
                    .com_hub()
                    .create_interface(
                        &connection.interface_type,
                        to_value_container(
                            &connection.setup_data.take().unwrap(),
                        )
                        .unwrap(),
                        connection.priority,
                    )
                    .await
                    .expect("failed to create interface");
            }

            runtime.start().await;
            endpoint.runtime = Some(runtime);
        }
    }

    pub fn get_runtime(&self, endpoint: impl Into<Endpoint>) -> &Runtime {
        let endpoint = endpoint.into();
        for node in self.endpoints.iter() {
            if node.endpoint == endpoint {
                return node.runtime.as_ref().unwrap();
            }
        }
        core::panic!("Endpoint {endpoint} not found in network");
    }
}
