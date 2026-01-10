use core::cell::RefCell;
use core::time::Duration;
use datex_core::task::{spawn_with_panic_notify, spawn_with_panic_notify_default, UnboundedSender};
use datex_core::values::core_values::endpoint::Endpoint;
use datex_core::{
    global::{
        dxb_block::DXBBlock, protocol_structures::block_header::BlockType,
    },
};
use datex_macros::{com_interface, create_opener};
use log::info;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, mpsc},
};
use std::collections::HashMap;
use std::fmt::Debug;
use datex_core::network::com_interfaces::com_interface::ComInterface;
use datex_core::network::com_interfaces::com_interface::error::ComInterfaceError;
use datex_core::network::com_interfaces::com_interface::implementation::{ComInterfaceFactory, ComInterfaceImplementation};
use datex_core::network::com_interfaces::com_interface::properties::{InterfaceDirection, InterfaceProperties};
use datex_core::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;

pub struct MockupInterface {
    pub outgoing_queue: RefCell<Vec<(ComInterfaceSocketUUID, Vec<u8>)>>,
    pub socket_senders: Rc<RefCell<HashMap<ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>>>>,
    pub sender: Option<mpsc::Sender<Vec<u8>>>,
    pub receiver: Rc<RefCell<Option<mpsc::Receiver<Vec<u8>>>>>,
    com_interface: Rc<RefCell<ComInterface>>,
    setup_data: MockupInterfaceSetupData,
}

impl Debug for MockupInterface {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MockupInterface")
            .field("outgoing_queue_length", &self.outgoing_queue.borrow().len())
            .field("socket_senders_count", &self.socket_senders.borrow().len())
            .field("has_sender", &self.sender.is_some())
            .field("has_receiver", &self.receiver.borrow().is_some())
            .field("setup_data", &self.setup_data)
            .finish()
    }
}

impl MockupInterface {
    pub fn new(
        setup_data: MockupInterfaceSetupData,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<Self, ComInterfaceError> {
        info!("Creating MockupInterface with setup data: {:?}", setup_data);
        let mut mockup_interface = MockupInterface {
            outgoing_queue: RefCell::new(Vec::new()),
            socket_senders: Rc::new(RefCell::new(HashMap::new())),
            receiver: Rc::new(RefCell::new(None)),
            sender: None,
            setup_data: setup_data.clone(),
            com_interface,
        };

        if let Some(sender) = setup_data.sender() {
            mockup_interface.sender = Some(sender);
        }
        if let Some(receiver) = setup_data.receiver() {
            mockup_interface.receiver = Rc::new(RefCell::new(Some(receiver)));
        }

        info!("MockupInterface created: {:?}", mockup_interface);

        let endpoint = setup_data.endpoint.clone();

        info!("endpoint: {endpoint:?}");

        mockup_interface.create_and_add_socket(endpoint)?;

        mockup_interface.start_update_loop();
        info!("started update loop");

        Ok(mockup_interface)
    }

    pub fn create_and_add_socket(&mut self, endpoint: Option<Endpoint>) -> Result<ComInterfaceSocketUUID, ComInterfaceError> {
        let direction = self.get_properties().direction.clone();
        let (socket_uuid, sender) = self
            .com_interface
            .borrow()
            .socket_manager()
            .lock()
            .unwrap()
            .create_and_init_socket(direction, 1);


        if let Some(endpoint) = endpoint {
            self
                .com_interface
                .borrow()
                .socket_manager()
                .lock()
                .unwrap()
                .register_socket_with_endpoint(
                    socket_uuid.clone(),
                    endpoint,
                    1,
                )?;
        }
        
        self.socket_senders.borrow_mut().insert(socket_uuid.clone(), sender);

        Ok(socket_uuid)
    }
}

type OptSender = Option<mpsc::Sender<Vec<u8>>>;
type OptReceiver = Option<mpsc::Receiver<Vec<u8>>>;

#[cfg_attr(not(feature = "embassy_runtime"), thread_local)]
pub static mut CHANNELS: Vec<(OptSender, OptReceiver)> = Vec::new();

pub fn store_sender_and_receiver(
    sender: OptSender,
    receiver: OptReceiver,
) -> usize {
    unsafe {
        CHANNELS.push((sender, receiver));
        CHANNELS.len() - 1
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MockupInterfaceSetupData {
    pub channel_index: Option<usize>,
    pub name: String,
    pub endpoint: Option<Endpoint>,
    pub direction: InterfaceDirection,
}

impl MockupInterfaceSetupData {
    pub fn new(name: &str) -> MockupInterfaceSetupData {
        MockupInterfaceSetupData {
            name: name.to_string(),
            channel_index: None,
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
            channel_index: None,
            endpoint: None,
            direction,
        }
    }
    pub fn new_with_endpoint(name: &str, endpoint: Endpoint) -> Self {
        MockupInterfaceSetupData {
            name: name.to_string(),
            channel_index: None,
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

    pub fn sender(&self) -> Option<mpsc::Sender<Vec<u8>>> {
        unsafe {
            if let Some(index) = self.channel_index {
                CHANNELS.get_mut(index).unwrap().0.take()
            } else {
                None
            }
        }
    }

    pub fn receiver(&self) -> Option<mpsc::Receiver<Vec<u8>>> {
        unsafe {
            if let Some(index) = self.channel_index {
                CHANNELS.get_mut(index).unwrap().1.take()
            } else {
                None
            }
        }
    }
}

impl ComInterfaceFactory for MockupInterface {
    type SetupData = MockupInterfaceSetupData;

    fn create(
        setup_data: Self::SetupData,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Result<MockupInterface, ComInterfaceError> {
        MockupInterface::new(setup_data, com_interface)
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

impl MockupInterface {
    pub fn last_block(&self) -> Option<Vec<u8>> {
        self.outgoing_queue.borrow().last().map(|(_, block)| block.clone())
    }
    pub fn last_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.outgoing_queue
            .borrow()
            .last()
            .map(|(socket_uuid, _)| socket_uuid.clone())
    }

    pub fn find_outgoing_block_for_socket(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Option<Vec<u8>> {
        self.outgoing_queue
            .borrow()
            .iter()
            .find(|(uuid, _)| uuid == socket_uuid)
            .map(|(_, block)| block.clone())
    }
    pub fn has_outgoing_block_for_socket(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> bool {
        self.find_outgoing_block_for_socket(socket_uuid).is_some()
    }

    pub fn last_block_and_socket(
        &self,
    ) -> Option<(ComInterfaceSocketUUID, Vec<u8>)> {
        self.outgoing_queue.borrow().last().cloned()
    }

    pub async fn update(&mut self) {
        MockupInterface::_update(
            self.receiver.clone(),
            self.socket_senders.clone()
        ).await;
    }

    pub async fn _update(
        receiver: Rc<RefCell<Option<mpsc::Receiver<Vec<u8>>>>>,
        socket_senders: Rc<RefCell<HashMap<ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>>>>,
    ) {
        if let Some(receiver) = &*receiver.borrow() {
            let mut socket_senders = socket_senders.borrow_mut();
            let sender = socket_senders.values_mut().next();
            if let Some(sender) = sender {
                while let Ok(block) = receiver.try_recv() {
                    sender.send(block).await.expect("Failed to send block to socket");
                }
            }
        }
    }

    pub fn start_update_loop(&mut self) {
        let receiver = self.receiver.clone();
        let sockets = self.socket_senders.clone();
        spawn_with_panic_notify_default(async move {
            loop {
                MockupInterface::_update(receiver.clone(), sockets.clone()).await;
                #[cfg(feature = "tokio_runtime")]
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
    }
}

impl ComInterfaceImplementation for MockupInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // FIXME #219 this should be inside the async body, why is it not working?
        Pin::from(Box::new(async move {
            let is_hello = {
                match DXBBlock::from_bytes(block).await {
                    Ok(block) => {
                        block.block_header.flags_and_timestamp.block_type()
                            == BlockType::Hello
                    }
                    _ => false,
                }
            };
            if !is_hello {
                self.outgoing_queue.borrow_mut().push((socket_uuid, block.to_vec()));
            }
            let mut result: bool = true;
            if let Some(sender) = &self.sender {
                if sender.send(block.to_vec()).is_err() {
                    result = false;
                }
            }
            result
        }))
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "mockup".to_string(),
            channel: "mockup".to_string(),
            name: Some(self.setup_data.name.clone()),
            direction: self.setup_data.direction.clone(),
            ..Default::default()
        }
    }

    fn handle_close<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        self.outgoing_queue.borrow_mut().clear();
        Pin::from(Box::new(async move { true }))
    }

    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output=bool> + 'a>> {
        Pin::from(Box::new(async move { true }))
    }
}
