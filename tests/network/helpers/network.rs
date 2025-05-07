use crate::context::init_global_context;
use crate::network::helpers::mockup_interface::MockupInterfaceSetupData;
use core::panic;
use datex_core::datex_values::Endpoint;
use datex_core::network::com_hub::{ComInterfaceFactoryFn, InterfacePriority};
use datex_core::network::com_interfaces::com_interface::ComInterfaceFactory;
use datex_core::network::com_interfaces::com_interface_properties::InterfaceDirection;
use datex_core::runtime::Runtime;
use log::info;
use serde::Deserialize;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;

use super::mockup_interface::MockupInterface;

pub struct InterfaceConnection {
    interface_type: String,
    priority: InterfacePriority,
    pub setup_data: Option<Box<dyn Any>>,
    pub endpoint: Option<Endpoint>,
}

impl InterfaceConnection {
    pub fn new<T: Any>(
        interface_type: &str,
        priority: InterfacePriority,
        setup_data: T,
    ) -> Self {
        InterfaceConnection {
            interface_type: interface_type.to_string(),
            priority,
            setup_data: Some(Box::new(setup_data)),
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
    pub runtime: Option<Runtime>,
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
    pub hops: Vec<(Endpoint, Option<String>)>,
}

impl Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (endpoint, channel)) in self.hops.iter().enumerate() {
            // Write the endpoint
            write!(f, "{endpoint}")?;

            // If not the last, write the arrow + optional channel
            if i + 1 < self.hops.len() {
                if let Some(chan) = channel {
                    write!(f, " -({chan})-> ")?;
                } else {
                    write!(f, " --> ")?;
                }
            }
        }
        Ok(())
    }
}

impl Route {
    pub fn from(
        source: impl Into<Endpoint>,
        receiver: impl Into<Endpoint>,
    ) -> Self {
        Route {
            receiver: receiver.into(),
            hops: vec![(source.into(), None)],
        }
    }

    pub async fn expect(&self, network: &Network) {
        let start = self.hops[0].0.clone();
        let end = self.hops.last().unwrap().0.clone();
        if start != end {
            panic!("Route start {} does not match receiver {}", start, end);
        }
        let network_trace = network
            .get_runtime(start)
            .com_hub
            .record_trace(self.receiver.clone())
            .await
            .expect("Failed to record trace");

        let mut index = 0;
        for (original, expected) in network_trace
            .hops
            .iter()
            .enumerate()
            .filter_map(
                |(i, h)| {
                    if i % 2 == 1 || i == 0 {
                        Some(h)
                    } else {
                        None
                    }
                },
            )
            .zip(self.hops.iter())
        {
            if original.endpoint != expected.0 {
                panic!(
                    "Expected hop #{} to be {} but was {}",
                    index, expected.0, original.endpoint
                );
            }
            if let Some(channel) = &expected.1 {
                if original.socket.interface_name != Some(channel.clone()) {
                    panic!(
                        "Expected hop #{} to be {} but was {}",
                        index,
                        channel,
                        original
                            .socket
                            .interface_name
                            .clone()
                            .unwrap_or("None".to_string())
                    );
                }
            }
            index += 1;
        }
    }

    pub fn to(mut self, target: impl Into<Endpoint>) -> Self {
        self.hops.push((target.into(), None));
        self
    }

    pub fn to_via(
        mut self,
        target: impl Into<Endpoint>,
        channel: &str,
    ) -> Self {
        let len = self.hops.len();
        if len > 0 {
            self.hops[len - 1].1 = Some(channel.to_string());
        }
        self.hops.push((target.into(), None));
        self
    }

    pub fn back(mut self) -> Self {
        if self.hops.len() >= 2 {
            let len = self.hops.len();
            let to = self.hops[len - 2].0.clone();
            let channel = self.hops[len - 2].1.clone();
            self.hops[len - 1].1 = Some(channel.clone().unwrap_or_default());
            self.hops.push((to, None));
        }
        self
    }
    pub fn back_via(mut self, channel: &str) -> Self {
        if self.hops.len() >= 2 {
            let len = self.hops.len();
            let to = self.hops[len - 2].0.clone();
            self.hops[len - 1].1 = Some(channel.to_string());
            self.hops.push((to, None));
        }
        self
    }

    /// Converts the Route into a sequence of (from, channel, to) triples
    pub fn to_segments(&self) -> Vec<(Endpoint, String, Endpoint)> {
        let mut segments = Vec::new();
        for w in self.hops.windows(2) {
            if let [(from, Some(chan)), (to, _)] = &w {
                segments.push((from.clone(), chan.clone(), to.clone()));
            }
        }
        segments
    }
}

#[test]
fn test() {
    init_global_context();
    // let route = Route::from("@aaa")
    //     .to_via("@bbb", "mockup")
    //     .to("@ccc")
    //     .back_via("mockup")
    //     .to("@ddd");
    // info!("Route: {}", route);
    // let segments = route.to_segments();
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
            let setup_data = setup_data
                .downcast_mut::<MockupInterfaceSetupData>()
                .expect("MockupInterfaceSetupData is required for interface of type mockup");
            let channel = Network::get_mockup_interface_channel(
                mockup_interface_channels,
                setup_data.name.clone(),
            );
            info!("setup_data: {:?}", setup_data.endpoint);
            info!("For Channel: {:?}", setup_data.name);

            match setup_data.direction {
                InterfaceDirection::In => {
                    setup_data.receiver = Some(channel.receiver);
                }
                InterfaceDirection::Out => {
                    setup_data.sender = Some(channel.sender);
                }
                InterfaceDirection::InOut => {
                    setup_data.receiver = Some(channel.receiver);
                    setup_data.sender = Some(channel.sender);
                }
            }
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
        } else if let Some(channel) =
            mockup_interface_channels.get_mut(&name).unwrap().take()
        {
            channel
        } else {
            panic!("Channel {name} is already used");
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
            panic!("Network already initialized");
        }
        self.is_initialized = true;

        // create new runtimes for each endpoint
        for endpoint in self.endpoints.iter_mut() {
            let runtime = Runtime::new(endpoint.endpoint.clone());

            // register factories
            for (interface_type, factory) in self.com_interface_factories.iter()
            {
                runtime.com_hub.register_interface_factory(
                    interface_type.clone(),
                    *factory,
                )
            }

            // add com interfaces
            for connection in endpoint.connections.iter_mut() {
                runtime
                    .com_hub
                    .create_interface(
                        &connection.interface_type,
                        connection.setup_data.take().unwrap(),
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
        panic!("Endpoint {endpoint} not found in network");
    }
}
