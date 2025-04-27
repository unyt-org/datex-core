use datex_core::datex_values::Endpoint;
use datex_core::global::dxb_block::DXBBlock;
use datex_core::network::com_hub::ComHub;
use datex_core::stdlib::cell::RefCell;
use datex_core::stdlib::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
// FIXME no-std
use datex_core::network::com_interfaces::com_interface::ComInterface;
use datex_core::network::com_interfaces::com_interface_properties::InterfaceDirection;
use datex_core::network::com_interfaces::com_interface_socket::ComInterfaceSocket;

use super::mockup_interface::MockupInterface;

lazy_static::lazy_static! {
    pub static ref ANY : Endpoint = Endpoint::ANY.clone();

    pub static ref ORIGIN : Endpoint = Endpoint::from_string("@origin").unwrap();
    pub static ref TEST_ENDPOINT_A: Endpoint = Endpoint::from_string("@test-a").unwrap();
    pub static ref TEST_ENDPOINT_B: Endpoint = Endpoint::from_string("@test-b").unwrap();
}

pub async fn get_mock_setup(
) -> (Arc<Mutex<ComHub>>, Rc<RefCell<MockupInterface>>) {
    get_mock_setup_with_endpoint(ORIGIN.clone()).await
}

pub async fn get_mock_setup_with_endpoint(
    endpoint: Endpoint,
) -> (Arc<Mutex<ComHub>>, Rc<RefCell<MockupInterface>>) {
    // init com hub
    let mut com_hub = ComHub::new(endpoint);

    // init mockup interface
    let mockup_interface_ref =
        Rc::new(RefCell::new(MockupInterface::default()));

    // add mockup interface to com_hub
    com_hub
        .open_and_add_interface(mockup_interface_ref.clone())
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {e:?}");
        });

    (Arc::new(Mutex::new(com_hub)), mockup_interface_ref.clone())
}

pub fn add_socket(
    mockup_interface_ref: Rc<RefCell<MockupInterface>>,
) -> Arc<Mutex<ComInterfaceSocket>> {
    let socket = Arc::new(Mutex::new(ComInterfaceSocket::new(
        mockup_interface_ref.borrow().get_uuid().clone(),
        InterfaceDirection::InOut,
        1,
    )));
    mockup_interface_ref.borrow().add_socket(socket.clone());
    socket
}

pub fn register_socket_endpoint(
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

pub async fn get_mock_setup_with_socket() -> (
    Arc<Mutex<ComHub>>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    get_mock_setup_with_socket_and_endpoint(
        ORIGIN.clone(),
        Some(TEST_ENDPOINT_A.clone()),
        None,
        None,
    )
    .await
}

pub async fn get_mock_setup_with_socket_and_endpoint(
    local_endpoint: Endpoint,
    remote_endpoint: Option<Endpoint>,
    sender: Option<mpsc::Sender<Vec<u8>>>,
    receiver: Option<mpsc::Receiver<Vec<u8>>>,
) -> (
    Arc<Mutex<ComHub>>,
    Rc<RefCell<MockupInterface>>,
    Arc<Mutex<ComInterfaceSocket>>,
) {
    let (com_hub, mockup_interface_ref) =
        get_mock_setup_with_endpoint(local_endpoint).await;

    mockup_interface_ref.borrow_mut().sender = sender;
    mockup_interface_ref.borrow_mut().receiver = receiver;

    let socket = add_socket(mockup_interface_ref.clone());
    if remote_endpoint.is_some() {
        register_socket_endpoint(
            mockup_interface_ref.clone(),
            socket.clone(),
            remote_endpoint.unwrap(),
        );
    }

    com_hub.lock().unwrap().update().await;

    (com_hub.clone(), mockup_interface_ref, socket)
}

pub async fn send_block_with_body(
    to: &[Endpoint],
    body: &[u8],
    com_hub: &Arc<Mutex<ComHub>>,
) -> DXBBlock {
    let mut com_hub_mut = com_hub.lock().unwrap();
    let mut block: DXBBlock = DXBBlock::default();
    block.set_receivers(to);
    block.body = body.to_vec();
    com_hub_mut.send_own_block(block.clone());
    com_hub_mut.update().await;
    block
}

pub async fn send_empty_block(
    to: &[Endpoint],
    com_hub: &Arc<Mutex<ComHub>>,
) -> DXBBlock {
    // send block
    let mut block: DXBBlock = DXBBlock::default();
    block.set_receivers(to);

    let mut com_hub_mut = com_hub.lock().unwrap();
    com_hub_mut.send_own_block(block.clone());
    com_hub_mut.update().await;
    block
}
