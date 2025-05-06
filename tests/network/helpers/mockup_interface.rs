use core::panic;
use datex_core::datex_values::Endpoint;
use datex_core::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceFactory,
};
use datex_core::network::com_interfaces::com_interface_properties::InterfaceDirection;
use datex_core::network::com_interfaces::com_interface_socket::ComInterfaceSocket;
use datex_core::task::spawn_local;
use datex_core::{
    delegate_com_interface_info,
    global::{
        dxb_block::DXBBlock, protocol_structures::block_header::BlockType,
    },
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceInfo, ComInterfaceSockets,
            ComInterfaceState, ComInterfaceUUID,
        },
        com_interface_properties::InterfaceProperties,
        com_interface_socket::ComInterfaceSocketUUID,
        socket_provider::SingleSocketProvider,
    },
    set_sync_opener,
};
use datex_macros::{com_interface, create_opener};
use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, Arc, Mutex},
};
use webrtc::media::audio::buffer::info;
use webrtc::mux::endpoint;
use webrtc::sdp::direction;

#[derive(Default)]
pub struct MockupInterface {
    pub outgoing_queue: Vec<(ComInterfaceSocketUUID, Vec<u8>)>,

    info: ComInterfaceInfo,
    pub sender: Option<mpsc::Sender<Vec<u8>>>,
    pub receiver: Rc<RefCell<Option<mpsc::Receiver<Vec<u8>>>>>,
}

impl MockupInterface {
    pub fn new(setup_data: MockupInterfaceSetupData) -> Self {
        let mut mockup_interface = MockupInterface::default();
        mockup_interface.info.interface_properties =
            Some(MockupInterface::get_default_properties());
        if let Some(interface_properties) =
            &mut mockup_interface.info.interface_properties
        {
            interface_properties.name = Some(setup_data.name.clone());
        }

        if let Some(sender) = setup_data.sender {
            mockup_interface.sender = Some(sender);
        }
        if let Some(receiver) = setup_data.receiver {
            mockup_interface.receiver = Rc::new(RefCell::new(Some(receiver)));
        }

        mockup_interface
    }

    pub fn init_socket(&mut self) -> Arc<Mutex<ComInterfaceSocket>> {
        let direction = self.get_properties().direction.clone();
        let socket = Arc::new(Mutex::new(ComInterfaceSocket::new(
            self.get_uuid().clone(),
            direction,
            1,
        )));
        self.add_socket(socket.clone());
        socket
    }
}

impl SingleSocketProvider for MockupInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

pub struct MockupInterfaceSetupData {
    pub sender: Option<mpsc::Sender<Vec<u8>>>,
    pub receiver: Option<mpsc::Receiver<Vec<u8>>>,
    pub name: String,
    pub endpoint: Option<Endpoint>,
    pub direction: InterfaceDirection,
}

impl MockupInterfaceSetupData {
    pub fn new(name: &str) -> MockupInterfaceSetupData {
        MockupInterfaceSetupData {
            name: name.to_string(),
            receiver: None,
            sender: None,
            endpoint: None,
            direction: InterfaceDirection::InOut,
        }
    }
    pub fn new_with_direction(
        name: &str,
        direction: InterfaceDirection,
    ) -> MockupInterfaceSetupData {
        MockupInterfaceSetupData {
            name: name.to_string(),
            receiver: None,
            sender: None,
            endpoint: None,
            direction,
        }
    }
    pub fn new_with_endpoint(name: &str, endpoint: Endpoint) -> Self {
        MockupInterfaceSetupData {
            name: name.to_string(),
            receiver: None,
            sender: None,
            endpoint: Some(endpoint),
            direction: InterfaceDirection::InOut,
        }
    }
    pub fn new_with_endpoint_and_direction(
        name: &str,
        endpoint: Endpoint,
        direction: InterfaceDirection,
    ) -> Self {
        let mut setup_data = Self::new_with_endpoint(name, endpoint);
        setup_data.direction = direction;
        setup_data
    }
}

impl ComInterfaceFactory<MockupInterfaceSetupData> for MockupInterface {
    fn create(
        setup_data: MockupInterfaceSetupData,
    ) -> Result<MockupInterface, ComInterfaceError> {
        let direction = setup_data.direction.clone();
        let endpoint = setup_data.endpoint.clone();
        let mut interface = MockupInterface::new(setup_data);

        let mut props = interface.init_properties();
        props.direction = direction;
        info!("props: {:?}", props.direction);
        info!("endpoint: {:?}", endpoint);
        interface.info.interface_properties = Some(props);
        interface.init_socket();
        if let Some(endpoint) = endpoint {
            interface
                .register_socket_endpoint(
                    interface.get_socket_uuid().clone().unwrap(),
                    endpoint,
                    1,
                )
                .unwrap();
        }
        interface.start_update_loop();
        log::info!("started update loop");
        Ok(interface)
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "mockup".to_string(),
            channel: "mockup".to_string(),
            name: Some("mockup".to_string()),
            ..Default::default()
        }
    }
}

#[com_interface]
impl MockupInterface {
    pub fn last_block(&self) -> Option<Vec<u8>> {
        self.outgoing_queue.last().map(|(_, block)| block.clone())
    }
    pub fn last_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.outgoing_queue
            .last()
            .map(|(socket_uuid, _)| socket_uuid.clone())
    }

    pub fn find_outgoing_block_for_socket(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Option<Vec<u8>> {
        self.outgoing_queue
            .iter()
            .find(|(uuid, _)| uuid == &socket_uuid)
            .map(|(_, block)| block.clone())
    }
    pub fn has_outgoing_block_for_socket(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> bool {
        self.find_outgoing_block_for_socket(socket_uuid).is_some()
    }

    pub fn last_block_and_socket(
        &self,
    ) -> Option<(ComInterfaceSocketUUID, Vec<u8>)> {
        self.outgoing_queue.last().cloned()
    }

    pub fn update(&mut self) {
        MockupInterface::_update(
            self.receiver.clone(),
            self.info.com_interface_sockets(),
        );
    }

    pub fn _update(
        receiver: Rc<RefCell<Option<mpsc::Receiver<Vec<u8>>>>>,
        sockets: Arc<Mutex<ComInterfaceSockets>>,
    ) {
        if let Some(receiver) = &*receiver.borrow() {
            let sockets = sockets.lock().unwrap();
            let socket = sockets.sockets.values().next();
            if let Some(socket) = socket {
                let socket = socket.lock().unwrap();
                let mut receive_queue = socket.receive_queue.lock().unwrap();
                while let Ok(block) = receiver.try_recv() {
                    receive_queue.extend(block);
                }
            }
        }
    }
    #[create_opener]
    fn open(&mut self) -> Result<(), ()> {
        Ok(())
    }

    pub fn start_update_loop(&mut self) {
        let receiver = self.receiver.clone();
        let sockets = self.info.com_interface_sockets();
        spawn_local(async move {
            loop {
                MockupInterface::_update(receiver.clone(), sockets.clone());
                #[cfg(feature = "tokio_runtime")]
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
    }
}

impl ComInterface for MockupInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // FIXME this should be inside the async body, why is it not working?
        let is_hello = {
            if let Ok(block) = DXBBlock::from_bytes(block) {
                block.block_header.flags_and_timestamp.block_type()
                    == BlockType::Hello
            } else {
                false
            }
        };
        if !is_hello {
            self.outgoing_queue.push((socket_uuid, block.to_vec()));
        }
        if let Some(sender) = &self.sender {
            if sender.send(block.to_vec()).is_err() {
                panic!("Failed to send block sender droppedß");
                return Pin::from(Box::new(async move { false }));
            }
        }
        Pin::from(Box::new(async move { true }))
    }

    fn init_properties(&self) -> InterfaceProperties {
        Self::get_default_properties()
    }

    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        self.outgoing_queue.clear();
        Pin::from(Box::new(async move { true }))
    }

    delegate_com_interface_info!();
    set_sync_opener!(open);
}
