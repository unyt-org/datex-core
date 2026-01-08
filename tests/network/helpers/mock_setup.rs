use super::mockup_interface::MockupInterface;
use core::str::FromStr;
use datex_core::global::dxb_block::{DXBBlock, IncomingSection};
use datex_core::network::com_hub::{ComHub, InterfacePriority};
use datex_core::network::com_interfaces::com_interface_old::ComInterfaceOld;
use datex_core::network::com_interfaces::com_interface_socket::ComInterfaceSocket;
use datex_core::runtime::{AsyncContext, Runtime, RuntimeConfig};
use datex_core::stdlib::cell::RefCell;
use datex_core::stdlib::rc::Rc;
use datex_core::values::core_values::endpoint::Endpoint;
use std::sync::{Arc, Mutex, mpsc};
use log::{error, info};
use tokio::task::yield_now;
use datex_core::network::block_handler::IncomingSectionsSinkType;

lazy_static::lazy_static! {
    pub static ref ANY : Endpoint = Endpoint::ANY.clone();

    pub static ref TEST_ENDPOINT_ORIGIN : Endpoint = Endpoint::from_str("@origin").unwrap();
    pub static ref TEST_ENDPOINT_A: Endpoint = Endpoint::from_str("@test-a").unwrap();
    pub static ref TEST_ENDPOINT_B: Endpoint = Endpoint::from_str("@test-b").unwrap();
    pub static ref TEST_ENDPOINT_C: Endpoint = Endpoint::from_str("@test-c").unwrap();
    pub static ref TEST_ENDPOINT_D: Endpoint = Endpoint::from_str("@test-d").unwrap();
    pub static ref TEST_ENDPOINT_E: Endpoint = Endpoint::from_str("@test-e").unwrap();
    pub static ref TEST_ENDPOINT_F: Endpoint = Endpoint::from_str("@test-f").unwrap();
    pub static ref TEST_ENDPOINT_G: Endpoint = Endpoint::from_str("@test-g").unwrap();
    pub static ref TEST_ENDPOINT_H: Endpoint = Endpoint::from_str("@test-h").unwrap();
    pub static ref TEST_ENDPOINT_I: Endpoint = Endpoint::from_str("@test-i").unwrap();
    pub static ref TEST_ENDPOINT_J: Endpoint = Endpoint::from_str("@test-j").unwrap();
    pub static ref TEST_ENDPOINT_K: Endpoint = Endpoint::from_str("@test-k").unwrap();
    pub static ref TEST_ENDPOINT_L: Endpoint = Endpoint::from_str("@test-l").unwrap();
    pub static ref TEST_ENDPOINT_M: Endpoint = Endpoint::from_str("@test-m").unwrap();
}

pub async fn get_mock_setup() -> (Rc<ComHub>, Rc<RefCell<MockupInterface>>) {
    get_mock_setup_with_endpoint(
        TEST_ENDPOINT_ORIGIN.clone(),
        InterfacePriority::default(),
        IncomingSectionsSinkType::Channel,
    )
    .await
}

pub async fn get_mock_setup_with_endpoint(
    endpoint: Endpoint,
    priority: InterfacePriority,
    sink_type: IncomingSectionsSinkType,
) -> (Rc<ComHub>, Rc<RefCell<MockupInterface>>) {
    // init com hub
    let com_hub = ComHub::create(endpoint, AsyncContext::new(), sink_type).await;

    // init mockup interface
    let mockup_interface_ref =
        Rc::new(RefCell::new(MockupInterface::default()));

    // add mockup interface to com_hub
    com_hub
        .open_and_add_interface(mockup_interface_ref.clone(), priority)
        .await
        .unwrap_or_else(|e| {
            core::panic!("Error adding interface: {e:?}");
        });

    (com_hub, mockup_interface_ref.clone())
}

pub async fn get_runtime_with_mock_interface(
    endpoint: Endpoint,
    priority: InterfacePriority,
) -> (Runtime, Rc<RefCell<MockupInterface>>) {
    // init com hub
    let runtime =
        Runtime::init_native(RuntimeConfig::new_with_endpoint(endpoint));

    // init mockup interface
    let mockup_interface_ref =
        Rc::new(RefCell::new(MockupInterface::default()));

    // add mockup interface to com_hub
    runtime
        .com_hub()
        .open_and_add_interface(mockup_interface_ref.clone(), priority)
        .await
        .unwrap_or_else(|e| {
            core::panic!("Error adding interface: {e:?}");
        });

    (runtime, mockup_interface_ref.clone())
}

pub fn create_and_add_socket(
    mockup_interface_ref: Rc<RefCell<MockupInterface>>,
) -> Arc<Mutex<ComInterfaceSocket>> {
    mockup_interface_ref.borrow_mut().create_and_add_socket()
}

pub fn register_socket_endpoint(
    mockup_interface_ref: Rc<RefCell<MockupInterface>>,
    socket: Arc<Mutex<ComInterfaceSocket>>,
    endpoint: Endpoint,
) {
    let mockup_interface = mockup_interface_ref.borrow_mut();
    let uuid = socket.try_lock().unwrap().uuid.clone();

    mockup_interface
        .register_socket_endpoint(uuid, endpoint, 1)
        .unwrap();
}

pub async fn get_mock_setup_and_socket(sink_type: IncomingSectionsSinkType) -> (
    Rc<ComHub>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    get_mock_setup_and_socket_for_endpoint(
        TEST_ENDPOINT_ORIGIN.clone(),
        Some(TEST_ENDPOINT_A.clone()),
        None,
        None,
        InterfacePriority::default(),
        sink_type
    )
    .await
}

pub async fn get_mock_setup_and_socket_for_priority(
    priority: InterfacePriority,
    sink_type: IncomingSectionsSinkType
) -> (
    Rc<ComHub>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    get_mock_setup_and_socket_for_endpoint(
        TEST_ENDPOINT_ORIGIN.clone(),
        Some(TEST_ENDPOINT_A.clone()),
        None,
        None,
        priority,
        sink_type
    )
    .await
}

pub async fn get_mock_setup_and_socket_for_endpoint(
    local_endpoint: Endpoint,
    remote_endpoint: Option<Endpoint>,
    sender: Option<mpsc::Sender<Vec<u8>>>,
    receiver: Option<mpsc::Receiver<Vec<u8>>>,
    priority: InterfacePriority,
    incoming_sections_sink_type: IncomingSectionsSinkType
) -> (
    Rc<ComHub>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    get_mock_setup_and_socket_for_endpoint_and_update_loop(
        local_endpoint,
        remote_endpoint,
        sender,
        receiver,
        priority,
        false,
        incoming_sections_sink_type
    )
    .await
}

pub async fn get_mock_setup_and_socket_for_endpoint_and_update_loop(
    local_endpoint: Endpoint,
    remote_endpoint: Option<Endpoint>,
    sender: Option<mpsc::Sender<Vec<u8>>>,
    receiver: Option<mpsc::Receiver<Vec<u8>>>,
    priority: InterfacePriority,
    enable_update_loop: bool,
    incoming_sections_sink_type: IncomingSectionsSinkType
) -> (
    Rc<ComHub>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    let (com_hub, mockup_interface_ref) =
        get_mock_setup_with_endpoint(local_endpoint, priority, incoming_sections_sink_type).await;

    mockup_interface_ref.borrow_mut().sender = sender;
    mockup_interface_ref.borrow_mut().receiver =
        Rc::new(RefCell::new(receiver));

    if enable_update_loop {
        // start mockup interface update loop
        mockup_interface_ref.borrow_mut().start_update_loop();

        tokio::task::yield_now().await;
    }

    let socket = create_and_add_socket(mockup_interface_ref.clone());
    
    if remote_endpoint.is_some() {
        register_socket_endpoint(
            mockup_interface_ref.clone(),
            socket.clone(),
            remote_endpoint.unwrap(),
        );
    }

    tokio::task::yield_now().await;

    (com_hub.clone(), mockup_interface_ref, socket)
}

pub async fn get_mock_setup_runtime(
    local_endpoint: Endpoint,
    sender: Option<mpsc::Sender<Vec<u8>>>,
    receiver: Option<mpsc::Receiver<Vec<u8>>>,
) -> Runtime {
    let (runtime, mockup_interface_ref) = get_runtime_with_mock_interface(
        local_endpoint,
        InterfacePriority::default(),
    )
    .await;

    mockup_interface_ref.borrow_mut().sender = sender;
    mockup_interface_ref.borrow_mut().receiver =
        Rc::new(RefCell::new(receiver));

    // start mockup interface update loop
    mockup_interface_ref.borrow_mut().start_update_loop();

    create_and_add_socket(mockup_interface_ref.clone());

    runtime.start().await;
    runtime
}

pub async fn get_mock_setup_with_two_runtimes(
    endpoint_a: Endpoint,
    endpoint_b: Endpoint,
) -> (Runtime, Runtime) {
    let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
    let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

    let runtime_a = get_mock_setup_runtime(
        endpoint_a.clone(),
        Some(sender_a),
        Some(receiver_b),
    )
    .await;

    let runtime_b = get_mock_setup_runtime(
        endpoint_b.clone(),
        Some(sender_b),
        Some(receiver_a),
    )
    .await;

    (runtime_a, runtime_b)
}

pub async fn send_block_with_body(
    to: &[Endpoint],
    body: &[u8],
    com_hub: &Rc<ComHub>,
) -> DXBBlock {
    let block = {
        let mut block: DXBBlock = DXBBlock::default();
        block.set_receivers(to);
        block.body = body.to_vec();
        com_hub.send_own_block(block.clone()).await.unwrap();
        block
    };

    yield_now().await;
    block
}

pub async fn send_empty_block_and_update(
    to: &[Endpoint],
    com_hub: &Rc<ComHub>,
) -> DXBBlock {
    let mut block: DXBBlock = DXBBlock::default();
    block.set_receivers(to);
    {
        if let Ok(sent_block) = com_hub.send_own_block(block.clone()).await {
            info!("Sent block: {:?}", sent_block);
        } else {
            error!("Failed to send block");
        }
    }

    yield_now().await;
    block
}

pub fn get_last_received_single_block_from_com_hub(
    com_hub: &ComHub,
) -> DXBBlock {
    let sections =
        com_hub.block_handler.drain_collected_sections();

    assert_eq!(sections.len(), 1);

    match &sections[0] {
        IncomingSection::SingleBlock((Some(block), ..)) => block.clone(),
        _ => {
            core::panic!("Expected single block, but got block stream");
        }
    }
}
pub fn get_all_received_single_blocks_from_com_hub(
    com_hub: &ComHub,
) -> Vec<DXBBlock> {
    let sections =
        com_hub.block_handler.drain_collected_sections();

    let mut blocks = vec![];

    for section in sections {
        match section {
            IncomingSection::SingleBlock((Some(block), ..)) => {
                blocks.push(block.clone());
            }
            _ => {
                core::panic!("Expected single block, but got block stream");
            }
        }
    }

    blocks
}
