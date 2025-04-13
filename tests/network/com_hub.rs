use datex_core::datex_values::Endpoint;
use datex_core::delegate_com_interface_info;
use datex_core::global::dxb_block::DXBBlock;
use datex_core::global::protocol_structures::encrypted_header::{
    self, EncryptedHeader,
};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::network::com_hub::ComHub;
use datex_core::stdlib::cell::RefCell;
use datex_core::stdlib::rc::Rc;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
// FIXME no-std

use datex_core::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
};
use datex_core::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use datex_core::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use datex_core::utils::uuid::UUID;

use crate::context::init_global_context;

lazy_static::lazy_static! {
    static ref ORIGIN : Endpoint = Endpoint::from_string("@origin").unwrap();
    static ref TEST_ENDPOINT_A: Endpoint = Endpoint::from_string("@test-a").unwrap();
    static ref TEST_ENDPOINT_B: Endpoint = Endpoint::from_string("@test-b").unwrap();
}

pub struct MockupInterface {
    pub block_queue: Vec<(ComInterfaceSocketUUID, Vec<u8>)>,
    info: ComInterfaceInfo,
}

impl MockupInterface {
    fn last_block(&self) -> Option<Vec<u8>> {
        self.block_queue.last().map(|(_, block)| block.clone())
    }
    fn last_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.block_queue
            .last()
            .map(|(socket_uuid, _)| socket_uuid.clone())
    }

    fn find_outgoing_block_for_socket(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Option<Vec<u8>> {
        self.block_queue
            .iter()
            .find(|(uuid, _)| uuid == &socket_uuid)
            .map(|(_, block)| block.clone())
    }
    fn has_outgoing_block_for_socket(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> bool {
        self.find_outgoing_block_for_socket(socket_uuid).is_some()
    }

    fn last_block_and_socket(
        &self,
    ) -> Option<(ComInterfaceSocketUUID, Vec<u8>)> {
        self.block_queue.last().cloned()
    }
}

impl Default for MockupInterface {
    fn default() -> Self {
        MockupInterface {
            block_queue: Vec::new(),
            info: ComInterfaceInfo::new(),
        }
    }
}

impl ComInterface for MockupInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // FIXME this should be inside the async body, why is it not working?
        self.block_queue.push((socket_uuid, block.to_vec()));

        Pin::from(Box::new(async move { true }))
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "mockup".to_string(),
            name: Some("mockup".to_string()),
            ..Default::default()
        }
    }

    delegate_com_interface_info!();
}

async fn get_mock_setup() -> (Rc<RefCell<ComHub>>, Rc<RefCell<MockupInterface>>)
{
    // init com hub
    let com_hub = ComHub::new(ORIGIN.clone());
    let mut com_hub_mut = com_hub.borrow_mut();

    // init mockup interface
    let mockup_interface_ref =
        Rc::new(RefCell::new(MockupInterface::default()));

    // add mockup interface to com_hub
    com_hub_mut
        .add_interface(mockup_interface_ref.clone())
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {:?}", e);
        });

    (com_hub.clone(), mockup_interface_ref.clone())
}

fn add_socket(
    mockup_interface_ref: Rc<RefCell<MockupInterface>>,
) -> Arc<Mutex<ComInterfaceSocket>> {
    let socket = Arc::new(Mutex::new(ComInterfaceSocket::new(
        mockup_interface_ref.borrow().get_uuid().clone(),
        InterfaceDirection::IN_OUT,
        1,
    )));
    socket.lock().unwrap().is_connected = true;
    mockup_interface_ref.borrow().add_socket(socket.clone());
    socket
}

fn register_socket_endpoint(
    mockup_interface_ref: Rc<RefCell<MockupInterface>>,
    socket: Arc<Mutex<ComInterfaceSocket>>,
    endpoint: Endpoint,
) {
    let mockup_interface = mockup_interface_ref.borrow_mut();
    let uuid = socket.lock().unwrap().uuid.clone();

    mockup_interface
        .register_socket_endpoint(uuid, endpoint, 1)
        .unwrap();
}

async fn get_mock_setup_with_socket() -> (
    Rc<RefCell<ComHub>>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    let (com_hub, mockup_interface_ref) = get_mock_setup().await;
    let mut com_hub_mut = com_hub.borrow_mut();

    let socket = add_socket(mockup_interface_ref.clone());
    register_socket_endpoint(
        mockup_interface_ref.clone(),
        socket.clone(),
        TEST_ENDPOINT_A.clone(),
    );

    com_hub_mut.update().await;

    (com_hub.clone(), mockup_interface_ref, socket)
}

#[tokio::test]
pub async fn test_add_and_remove() {
    init_global_context();
    let com_hub = Rc::new(RefCell::new(ComHub::default()));
    let mut com_hub_mut = com_hub.borrow_mut();
    let uuid = {
        let mockup_interface =
            Rc::new(RefCell::new(MockupInterface::default()));
        let uuid = mockup_interface.borrow().get_uuid().clone();
        com_hub_mut
            .add_interface(mockup_interface.clone())
            .await
            .unwrap_or_else(|e| {
                panic!("Error adding interface: {:?}", e);
            });
        uuid
    };
    assert!(com_hub_mut.remove_interface(uuid).await.is_ok());
}

#[tokio::test]
pub async fn test_multiple_add() {
    init_global_context();

    let com_hub = Rc::new(RefCell::new(ComHub::default()));
    let mut com_hub_mut = com_hub.borrow_mut();

    let mockup_interface1 = Rc::new(RefCell::new(MockupInterface::default()));
    let mockup_interface2 = Rc::new(RefCell::new(MockupInterface::default()));

    com_hub_mut
        .add_interface(mockup_interface1.clone())
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {:?}", e);
        });
    com_hub_mut
        .add_interface(mockup_interface2.clone())
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {:?}", e);
        });

    assert!(com_hub_mut
        .add_interface(mockup_interface1.clone())
        .await
        .is_err());
    assert!(com_hub_mut
        .add_interface(mockup_interface2.clone())
        .await
        .is_err());
}

async fn send_empty_block(
    endpoints: &[Endpoint],
    com_hub: &Rc<RefCell<ComHub>>,
) -> DXBBlock {
    // send block
    let mut block: DXBBlock = DXBBlock::default();
    block.set_receivers(endpoints);

    let mut com_hub_mut = com_hub.borrow_mut();
    com_hub_mut.send_block(&block, None);
    com_hub_mut.update().await;
    block
}

#[tokio::test]
pub async fn test_send() {
    // init mock setup
    init_global_context();
    let (com_hub, com_interface, _) = get_mock_setup_with_socket().await;

    let block = send_empty_block(&[TEST_ENDPOINT_A.clone()], &com_hub).await;

    // get last block that was sent
    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    let block_bytes = mockup_interface_out.last_block().unwrap();

    assert!(mockup_interface_out.last_block().is_some());
    assert_eq!(block_bytes, block.to_bytes().unwrap());
}

#[tokio::test]
pub async fn test_send_invalid_recipient() {
    // init mock setup
    init_global_context();
    let (com_hub, com_interface, _) = get_mock_setup_with_socket().await;

    send_empty_block(&[TEST_ENDPOINT_B.clone()], &com_hub).await;

    // get last block that was sent
    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();

    assert!(mockup_interface_out.last_block().is_none());
}

#[tokio::test]
pub async fn send_block_to_multiple_endpoints() {
    // init mock setup
    init_global_context();
    let (com_hub, com_interface) = get_mock_setup().await;

    let socket = add_socket(com_interface.clone());
    register_socket_endpoint(
        com_interface.clone(),
        socket.clone(),
        TEST_ENDPOINT_A.clone(),
    );
    register_socket_endpoint(
        com_interface.clone(),
        socket.clone(),
        TEST_ENDPOINT_B.clone(),
    );
    com_hub.borrow_mut().update().await;

    // send block to multiple receivers
    let block = send_empty_block(
        &[TEST_ENDPOINT_A.clone(), TEST_ENDPOINT_B.clone()],
        &com_hub,
    )
    .await;

    // get last block that was sent
    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    let block_bytes = mockup_interface_out.last_block().unwrap();

    assert_eq!(mockup_interface_out.block_queue.len(), 1);
    assert!(mockup_interface_out.last_block().is_some());
    assert_eq!(block_bytes, block.to_bytes().unwrap());
}

#[tokio::test]
pub async fn send_blocks_to_multiple_endpoints() {
    init_global_context();
    let (com_hub, com_interface) = get_mock_setup().await;

    let socket_a = add_socket(com_interface.clone());
    let socket_b = add_socket(com_interface.clone());
    register_socket_endpoint(
        com_interface.clone(),
        socket_a.clone(),
        TEST_ENDPOINT_A.clone(),
    );
    register_socket_endpoint(
        com_interface.clone(),
        socket_b.clone(),
        TEST_ENDPOINT_B.clone(),
    );
    com_hub.borrow_mut().update().await;

    // send block to multiple receivers
    let _ = send_empty_block(
        &[TEST_ENDPOINT_A.clone(), TEST_ENDPOINT_B.clone()],
        &com_hub,
    )
    .await;

    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    assert_eq!(mockup_interface_out.block_queue.len(), 2);

    assert!(mockup_interface_out
        .has_outgoing_block_for_socket(socket_a.lock().unwrap().uuid.clone()));
    assert!(mockup_interface_out
        .has_outgoing_block_for_socket(socket_b.lock().unwrap().uuid.clone()));

    assert!(mockup_interface_out.last_block().is_some());
}

#[tokio::test]
pub async fn default_interface_create_socket_first() {
    init_global_context();
    let (com_hub, com_interface, _) = get_mock_setup_with_socket().await;

    com_hub
        .borrow_mut()
        .set_default_interface(com_interface.borrow().get_uuid().clone())
        .unwrap_or_else(|e| {
            panic!("Error setting default interface: {:?}", e);
        });

    let _ = send_empty_block(&[TEST_ENDPOINT_B.clone()], &com_hub).await;

    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    assert_eq!(mockup_interface_out.block_queue.len(), 1);
}

#[tokio::test]
pub async fn default_interface_set_default_interface_first() {
    init_global_context();
    let (com_hub, com_interface) = get_mock_setup().await;

    com_hub
        .borrow_mut()
        .set_default_interface(com_interface.borrow().get_uuid().clone())
        .unwrap_or_else(|e| {
            panic!("Error setting default interface: {:?}", e);
        });

    let socket = add_socket(com_interface.clone());
    register_socket_endpoint(
        com_interface.clone(),
        socket.clone(),
        TEST_ENDPOINT_A.clone(),
    );

    // Update to let the com_hub know about the socket and call the add_socket method
    // This will set the default interface and socket
    com_hub.borrow_mut().update().await;

    let _ = send_empty_block(&[TEST_ENDPOINT_B.clone()], &com_hub).await;

    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    assert_eq!(mockup_interface_out.block_queue.len(), 1);
}

#[test]
pub fn test_recalculate() {
    init_global_context();

    let mut block = DXBBlock {
        body: vec![0x01, 0x02, 0x03],
        encrypted_header: EncryptedHeader {
            flags: encrypted_header::Flags::new()
                .with_device_type(encrypted_header::DeviceType::Unused11),
            ..Default::default()
        },
        routing_header: RoutingHeader {
            block_size_u16: Some(420),
            sender: Endpoint::from_string("@test").unwrap(),
            ..Default::default()
        },
        ..DXBBlock::default()
    };

    {
        // invalid block size
        let block_bytes = block.to_bytes().unwrap();
        let block2: DXBBlock = DXBBlock::from_bytes(&block_bytes).unwrap();
        assert_ne!(block, block2);
    }

    {
        // valid block size
        block.recalculate_struct();
        let block_bytes = block.to_bytes().unwrap();
        let block3: DXBBlock = DXBBlock::from_bytes(&block_bytes).unwrap();
        assert_eq!(block, block3);
    }
}

#[tokio::test]
pub async fn test_receive() {
    // init mock setup
    init_global_context();
    let (com_hub, _, socket) = get_mock_setup_with_socket().await;
    let mut com_hub_mut = com_hub.borrow_mut();

    // receive block
    let mut block = DXBBlock {
        body: vec![0x01, 0x02, 0x03],
        encrypted_header: EncryptedHeader {
            flags: encrypted_header::Flags::new()
                .with_device_type(encrypted_header::DeviceType::Unused11),
            ..Default::default()
        },
        ..DXBBlock::default()
    };
    block.set_receivers(&[ORIGIN.clone()]);
    block.recalculate_struct();

    let block_bytes = block.to_bytes().unwrap();
    {
        let socket_ref = socket.lock().unwrap();
        let receive_queue = socket_ref.get_receive_queue();
        let mut receive_queue_mut = receive_queue.lock().unwrap();
        let _ = receive_queue_mut.write(block_bytes.as_slice());
    }

    com_hub_mut.update().await;

    let incoming_blocks_ref = com_hub_mut.incoming_blocks.clone();
    let incoming_blocks = incoming_blocks_ref.borrow();

    assert_eq!(incoming_blocks.len(), 1);
    let incoming_block = incoming_blocks.front().unwrap();
    assert_eq!(incoming_block.raw_bytes.clone().unwrap(), block_bytes);
}

#[tokio::test]
pub async fn test_receive_multiple() {
    // init mock setup
    init_global_context();
    let (com_hub, _, socket) = get_mock_setup_with_socket().await;
    let mut com_hub_mut = com_hub.borrow_mut();

    // receive block
    let blocks = vec![
        DXBBlock {
            routing_header: RoutingHeader {
                block_index: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        DXBBlock {
            routing_header: RoutingHeader {
                block_index: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        DXBBlock {
            routing_header: RoutingHeader {
                block_index: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    let block_bytes: Vec<Vec<u8>> = blocks
        .iter()
        .map(|block| block.to_bytes().unwrap())
        .collect();

    {
        let socket_ref = socket.lock().unwrap();
        let receive_queue = socket_ref.get_receive_queue();
        let mut receive_queue_mut = receive_queue.lock().unwrap();
        for block in block_bytes.iter() {
            let _ = receive_queue_mut.write(block);
        }
    }

    com_hub_mut.update().await;

    let incoming_blocks_ref = com_hub_mut.incoming_blocks.clone();
    let incoming_blocks = incoming_blocks_ref.borrow();

    assert_eq!(incoming_blocks.len(), blocks.len());

    for (incoming_block, block) in incoming_blocks.iter().zip(blocks.iter()) {
        assert_eq!(
            incoming_block.raw_bytes.clone().unwrap(),
            block.to_bytes().unwrap()
        );
    }
}

#[test]
pub fn test_send_receive() {}
