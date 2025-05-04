use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc;
use datex_core::datex_values::Endpoint;
use datex_core::network::com_hub::{ComInterfaceFactoryFn, InterfacePriority};
use datex_core::runtime::Runtime;
use crate::network::helpers::mockup_interface::MockupInterfaceSetupData;

pub struct InterfaceConnection {
    interface_type: String,
    priority: InterfacePriority,
    pub setup_data: Option<Box<dyn Any>>,
    pub endpoint: Option<Endpoint>,
}

impl InterfaceConnection {
    pub fn new<T: Any>(interface_type: &str, priority: InterfacePriority, setup_data: T) -> Self {
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

type MockupInterfaceChannels = HashMap<
    String,
    Option<MockupInterfaceChannelEndpoint>
>;

pub struct Network {
    pub is_initialized: bool,
    pub endpoints: Vec<Node>,
    com_interface_factories: HashMap<String, ComInterfaceFactoryFn>,
}

impl Network {

    pub fn create(mut endpoints: Vec<Node>) -> Self {
        let mut mockup_interface_channels = HashMap::new();

        // iterate over all endpoints and handle mockup endpoints
        for endpoint in endpoints.iter_mut() {
            for connection in endpoint.connections.iter_mut() {
                if connection.interface_type == "mockup" {
                    Network::init_mockup_endpoint(
                        connection,
                        &mut mockup_interface_channels
                    );
                }
            }
        }

        Network {
            is_initialized: false,
            endpoints,
            com_interface_factories: HashMap::new(),
        }
    }

    fn init_mockup_endpoint(
        connection: &mut InterfaceConnection,
        mut mockup_interface_channels: &mut MockupInterfaceChannels
    ) {
        // get setup data as MockupInterfaceSetupData
        if let Some(setup_data) = &mut connection.setup_data {
            let setup_data = setup_data
                .downcast_mut::<MockupInterfaceSetupData>()
                .expect("MockupInterfaceSetupData is required for interface of type mockup");
            let channel = Network::get_mockup_interface_channel(
                mockup_interface_channels,
                setup_data.name.clone()
            );
            setup_data.receiver = Some(channel.receiver);
            setup_data.sender = Some(channel.sender);
        }
    }

    fn get_mockup_interface_channel(mockup_interface_channels: &mut MockupInterfaceChannels, name: String) -> MockupInterfaceChannelEndpoint {
        if !mockup_interface_channels.contains_key(&name) {
            let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
            let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

            mockup_interface_channels.insert(
                name,
                Some(MockupInterfaceChannelEndpoint {
                    sender: sender_b,
                    receiver: receiver_a
                })
            );

            MockupInterfaceChannelEndpoint {
                sender: sender_a,
                receiver: receiver_b
            }
        }

        else if let Some(channel) = mockup_interface_channels.get_mut(&name).unwrap().take() {
            channel
        }

        else {
            panic!("Channel {name} is already used");
        }

    }

    pub fn register_interface(&mut self, interface_type: &str, factory: ComInterfaceFactoryFn) {
        self.com_interface_factories.insert(interface_type.to_string(), factory);
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
            for (interface_type, factory) in self.com_interface_factories.iter() {
                runtime.com_hub.register_interface_factory(interface_type.clone(), *factory)
            }

            // add com interfaces
            for connection in endpoint.connections.iter_mut() {
                runtime.com_hub.create_interface(
                    &connection.interface_type,
                    connection.setup_data.take().unwrap(),
                    connection.priority
                ).await.expect("failed to create interface");
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