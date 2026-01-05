use super::mockup_interface::MockupInterface;
use core::str::FromStr;
use datex_core::global::dxb_block::{DXBBlock, IncomingSection};
use datex_core::network::com_hub::{ComHub, InterfacePriority};
use datex_core::network::com_interfaces::com_interface::ComInterface;
use datex_core::network::com_interfaces::com_interface_socket::ComInterfaceSocket;
use datex_core::runtime::{AsyncContext, Runtime, RuntimeConfig};
use datex_core::stdlib::cell::RefCell;
use datex_core::stdlib::rc::Rc;
use datex_core::values::core_values::endpoint::Endpoint;
use std::sync::{Arc, Mutex, mpsc};

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
    )
    .await
}

pub async fn get_mock_setup_with_endpoint(
    endpoint: Endpoint,
    priority: InterfacePriority,
) -> (Rc<ComHub>, Rc<RefCell<MockupInterface>>) {
    // init com hub
    let com_hub = ComHub::new(endpoint, AsyncContext::new());

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

    (Rc::new(com_hub), mockup_interface_ref.clone())
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

pub fn add_socket(
    mockup_interface_ref: Rc<RefCell<MockupInterface>>,
) -> Arc<Mutex<ComInterfaceSocket>> {
    mockup_interface_ref.borrow_mut().init_socket_default()
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

pub async fn get_mock_setup_and_socket() -> (
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
    )
    .await
}

pub async fn get_mock_setup_and_socket_for_priority(
    priority: InterfacePriority,
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
    )
    .await
}

pub async fn get_mock_setup_and_socket_for_endpoint(
    local_endpoint: Endpoint,
    remote_endpoint: Option<Endpoint>,
    sender: Option<mpsc::Sender<Vec<u8>>>,
    receiver: Option<mpsc::Receiver<Vec<u8>>>,
    priority: InterfacePriority,
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
) -> (
    Rc<ComHub>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    let (com_hub, mockup_interface_ref) =
        get_mock_setup_with_endpoint(local_endpoint, priority).await;

    mockup_interface_ref.borrow_mut().sender = sender;
    mockup_interface_ref.borrow_mut().receiver =
        Rc::new(RefCell::new(receiver));

    if enable_update_loop {
        ComHub::_start_update_loop(com_hub.clone());

        // start mockup interface update loop
        mockup_interface_ref.borrow_mut().start_update_loop();

        tokio::task::yield_now().await;
    }

    let socket = add_socket(mockup_interface_ref.clone());
    if remote_endpoint.is_some() {
        register_socket_endpoint(
            mockup_interface_ref.clone(),
            socket.clone(),
            remote_endpoint.unwrap(),
        );
    }

    if !enable_update_loop {
        com_hub.update_async().await;
    } else {
        tokio::task::yield_now().await;
    }

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

    add_socket(mockup_interface_ref.clone());

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

    com_hub.update_async().await;
    block
}

pub async fn send_empty_block_and_update(
    to: &[Endpoint],
    com_hub: &Rc<ComHub>,
) -> DXBBlock {
    let mut block: DXBBlock = DXBBlock::default();
    block.set_receivers(to);
    {
        com_hub.send_own_block(block.clone()).await;
    }
    com_hub.update_async().await;
    block
}

pub fn get_last_received_single_block_from_com_hub(
    com_hub: &ComHub,
) -> DXBBlock {
    let mut sections =
        com_hub.block_handler.incoming_sections_queue.borrow_mut();
    let sections = sections.drain(..).collect::<Vec<_>>();

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
    let mut sections =
        com_hub.block_handler.incoming_sections_queue.borrow_mut();
    let sections = sections.drain(..).collect::<Vec<_>>();

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
