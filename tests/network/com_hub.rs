use datex_core::datex_values::Endpoint;
use datex_core::global::dxb_block::DXBBlock;
use datex_core::global::protocol_structures::block_header::BlockHeader;
use datex_core::global::protocol_structures::encrypted_header::{
    self, EncryptedHeader,
};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::network::com_hub::ComHub;
use datex_core::stdlib::cell::RefCell;
use datex_core::stdlib::rc::Rc;
use std::io::Write;
use std::str::FromStr;
use std::sync::mpsc;
// FIXME no-std
use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{
    add_socket, get_mock_setup, get_mock_setup_with_socket,
    register_socket_endpoint, send_block_with_body, send_empty_block, ORIGIN,
    TEST_ENDPOINT_A, TEST_ENDPOINT_B,
};
use crate::network::helpers::mockup_interface::MockupInterface;
use datex_core::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceFactory, ComInterfaceState,
};
use datex_core::network::com_interfaces::com_interface_socket::SocketState;

use super::helpers::mock_setup::get_mock_setup_with_socket_and_endpoint;

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
            .open_and_add_interface(mockup_interface.clone())
            .await
            .unwrap_or_else(|e| {
                panic!("Error adding interface: {e:?}");
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
        .open_and_add_interface(mockup_interface1.clone())
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {e:?}");
        });
    com_hub_mut
        .open_and_add_interface(mockup_interface2.clone())
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {e:?}");
        });

    assert!(com_hub_mut
        .open_and_add_interface(mockup_interface1.clone())
        .await
        .is_err());
    assert!(com_hub_mut
        .open_and_add_interface(mockup_interface2.clone())
        .await
        .is_err());
}

#[tokio::test]
pub async fn test_send() {
    // init mock setup
    init_global_context();
    let (com_hub, com_interface, _) = get_mock_setup_with_socket().await;

    let block = send_block_with_body(
        &[TEST_ENDPOINT_A.clone()],
        b"Hello world!",
        &com_hub,
    )
    .await;

    // get last block that was sent
    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    let block_bytes =
        DXBBlock::from_bytes(&mockup_interface_out.last_block().unwrap())
            .unwrap();

    assert!(mockup_interface_out.last_block().is_some());
    assert_eq!(block_bytes.body, block.body);
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
    com_hub.lock().unwrap().update().await;

    // send block to multiple receivers
    let block = send_block_with_body(
        &[TEST_ENDPOINT_A.clone(), TEST_ENDPOINT_B.clone()],
        b"Hello world",
        &com_hub,
    )
    .await;

    // get last block that was sent
    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    let block_bytes =
        DXBBlock::from_bytes(&mockup_interface_out.last_block().unwrap())
            .unwrap();

    assert_eq!(mockup_interface_out.outgoing_queue.len(), 1);
    assert!(mockup_interface_out.last_block().is_some());
    assert_eq!(block_bytes.body, block.body);
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
    com_hub.lock().unwrap().update().await;

    // send block to multiple receivers
    let _ = send_empty_block(
        &[TEST_ENDPOINT_A.clone(), TEST_ENDPOINT_B.clone()],
        &com_hub,
    )
    .await;

    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    assert_eq!(mockup_interface_out.outgoing_queue.len(), 2);

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
        .lock()
        .unwrap()
        .set_default_interface(com_interface.borrow().get_uuid().clone())
        .unwrap_or_else(|e| {
            panic!("Error setting default interface: {e:?}");
        });

    let _ = send_empty_block(&[TEST_ENDPOINT_B.clone()], &com_hub).await;

    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    assert_eq!(mockup_interface_out.outgoing_queue.len(), 1);
}

#[tokio::test]
pub async fn default_interface_set_default_interface_first() {
    init_global_context();
    let (com_hub, com_interface) = get_mock_setup().await;

    com_hub
        .lock()
        .unwrap()
        .set_default_interface(com_interface.borrow().get_uuid().clone())
        .unwrap_or_else(|e| {
            panic!("Error setting default interface: {e:?}");
        });

    let socket = add_socket(com_interface.clone());
    register_socket_endpoint(
        com_interface.clone(),
        socket.clone(),
        TEST_ENDPOINT_A.clone(),
    );

    // Update to let the com_hub know about the socket and call the add_socket method
    // This will set the default interface and socket
    com_hub.lock().unwrap().update().await;

    let _ = send_empty_block(&[TEST_ENDPOINT_B.clone()], &com_hub).await;

    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    assert_eq!(mockup_interface_out.outgoing_queue.len(), 1);
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
            sender: Endpoint::from_str("@test").unwrap(),
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
    let mut com_hub_mut = com_hub.lock().unwrap();
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

    // receive block
    let mut blocks = vec![
        DXBBlock {
            routing_header: RoutingHeader {
                ..Default::default()
            },
            block_header: BlockHeader {
                block_index: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        DXBBlock {
            routing_header: RoutingHeader {
                ..Default::default()
            },
            block_header: BlockHeader {
                block_index: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        DXBBlock {
            routing_header: RoutingHeader {
                ..Default::default()
            },
            block_header: BlockHeader {
                block_index: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    ];

    for block in &mut blocks {
        // set receiver to ORIGIN
        block.set_receivers(&[ORIGIN.clone()]);
    }

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

    let mut com_hub_mut = com_hub.lock().unwrap();
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

#[tokio::test]
pub async fn test_add_and_remove_interface_and_sockets() {
    init_global_context();

    let (com_hub_mut, com_interface, socket) =
        get_mock_setup_with_socket().await;

    let mut com_hub_mut = com_hub_mut.lock().unwrap();
    assert_eq!(com_hub_mut.interfaces.len(), 1);
    assert_eq!(com_hub_mut.sockets.len(), 1);
    assert_eq!(com_hub_mut.endpoint_sockets.len(), 1);

    assert_eq!(
        com_interface.borrow_mut().get_info().get_state(),
        ComInterfaceState::Connected
    );

    assert_eq!(socket.lock().unwrap().state, SocketState::Open);

    let uuid = com_interface.borrow().get_uuid().clone();

    // remove interface
    assert!(com_hub_mut.remove_interface(uuid).await.is_ok());

    assert_eq!(com_hub_mut.interfaces.len(), 0);
    assert_eq!(com_hub_mut.sockets.len(), 0);
    assert_eq!(com_hub_mut.endpoint_sockets.len(), 0);

    assert_eq!(
        com_interface.borrow_mut().get_info().get_state(),
        ComInterfaceState::Destroyed
    );

    assert_eq!(socket.lock().unwrap().state, SocketState::Destroyed);
}

#[tokio::test]
pub async fn test_basic_routing() {
    init_global_context();
    let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
    let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

    let (com_hub_mut_a, com_interface_a, socket_a) =
        get_mock_setup_with_socket_and_endpoint(
            TEST_ENDPOINT_A.clone(),
            None,
            Some(sender_a),
            Some(receiver_b),
        )
        .await;

    let (com_hub_mut_b, com_interface_b, socket_b) =
        get_mock_setup_with_socket_and_endpoint(
            TEST_ENDPOINT_B.clone(),
            None,
            Some(sender_b),
            Some(receiver_a),
        )
        .await;

    com_interface_a.borrow_mut().update();
    com_interface_b.borrow_mut().update();

    com_hub_mut_a.lock().unwrap().update().await;
    com_hub_mut_b.lock().unwrap().update().await;

    let block_a_to_b = send_block_with_body(
        &[TEST_ENDPOINT_B.clone()],
        b"Hello world",
        &com_hub_mut_a,
    )
    .await;

    com_interface_b.borrow_mut().update();
    com_hub_mut_b.lock().unwrap().update().await;

    let blocks = com_hub_mut_b
        .lock()
        .unwrap()
        .incoming_blocks
        .borrow_mut()
        .drain(..)
        .collect::<Vec<_>>();

    assert_eq!(blocks.len(), 1);
    assert_eq!(block_a_to_b.body, blocks[0].body);
}

#[tokio::test]
pub async fn register_factory() {
    init_global_context();
    let mut com_hub = ComHub::default();
    MockupInterface::register_on_com_hub(&mut com_hub);

    assert_eq!(com_hub.interface_factories.len(), 1);
    assert!(com_hub.interface_factories.get("mockup").is_some());

    // create a new mockup interface from the com_hub
    let mockup_interface = com_hub
        .create_interface("mockup", Box::new(()))
        .await
        .unwrap();

    assert_eq!(
        mockup_interface
            .borrow_mut()
            .get_properties()
            .interface_type,
        "mockup"
    );
}
