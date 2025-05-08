use datex_core::datex_values::Endpoint;
use datex_core::global::dxb_block::DXBBlock;
use datex_core::global::protocol_structures::block_header::BlockHeader;
use datex_core::global::protocol_structures::encrypted_header::{
    self, EncryptedHeader,
};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::network::com_hub::{ComHub, InterfacePriority};
use datex_core::network::com_interfaces::com_interface_properties::{InterfaceProperties, ReconnectionConfig};
use datex_core::network::com_interfaces::default_com_interfaces::base_interface::BaseInterface;
use datex_core::run_async;
use datex_core::stdlib::cell::RefCell;
use datex_core::stdlib::rc::Rc;
use itertools::Itertools;
use core::panic;
use std::io::Write;
use std::str::FromStr;
use std::sync::mpsc;
use tokio::task;
// FIXME no-std
use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{
    add_socket, get_all_received_single_blocks_from_com_hub,
    get_last_received_single_block_from_com_hub, get_mock_setup,
    get_mock_setup_and_socket, get_mock_setup_and_socket_for_priority,
    get_mock_setup_with_endpoint, register_socket_endpoint,
    send_block_with_body, send_empty_block_and_update, ORIGIN, TEST_ENDPOINT_A,
    TEST_ENDPOINT_B,
};
use crate::network::helpers::mockup_interface::{
    MockupInterface, MockupInterfaceSetupData,
};
use datex_core::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceFactory, ComInterfaceState,
};
use datex_core::network::com_interfaces::com_interface_socket::SocketState;

use super::helpers::mock_setup::get_mock_setup_and_socket_for_endpoint;

#[tokio::test]
pub async fn test_add_and_remove() {
    init_global_context();
    let com_hub = Rc::new(ComHub::default());
    let uuid = {
        let mockup_interface =
            Rc::new(RefCell::new(MockupInterface::default()));
        let uuid = mockup_interface.borrow().get_uuid().clone();
        com_hub
            .open_and_add_interface(
                mockup_interface.clone(),
                InterfacePriority::default(),
            )
            .await
            .unwrap_or_else(|e| {
                panic!("Error adding interface: {e:?}");
            });
        uuid
    };
    assert!(com_hub.remove_interface(uuid).await.is_ok());
}

#[tokio::test]
pub async fn test_multiple_add() {
    init_global_context();

    let com_hub = Rc::new(RefCell::new(ComHub::default()));
    let com_hub_mut = com_hub.borrow_mut();

    let mockup_interface1 = Rc::new(RefCell::new(MockupInterface::default()));
    let mockup_interface2 = Rc::new(RefCell::new(MockupInterface::default()));

    com_hub_mut
        .open_and_add_interface(
            mockup_interface1.clone(),
            InterfacePriority::default(),
        )
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {e:?}");
        });
    com_hub_mut
        .open_and_add_interface(
            mockup_interface2.clone(),
            InterfacePriority::default(),
        )
        .await
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {e:?}");
        });

    assert!(com_hub_mut
        .open_and_add_interface(
            mockup_interface1.clone(),
            InterfacePriority::default()
        )
        .await
        .is_err());
    assert!(com_hub_mut
        .open_and_add_interface(
            mockup_interface2.clone(),
            InterfacePriority::default()
        )
        .await
        .is_err());
}

#[tokio::test]
pub async fn test_send() {
    run_async! {
        // init mock setup
        init_global_context();
        let (com_hub, com_interface, _) = get_mock_setup_and_socket().await;

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
}

#[tokio::test]
pub async fn test_send_invalid_recipient() {
    run_async! {
        // init mock setup
        init_global_context();
        // init without fallback interfaces
        let (com_hub, com_interface, _) =
            get_mock_setup_and_socket_for_priority(InterfacePriority::None).await;

        send_empty_block_and_update(&[TEST_ENDPOINT_B.clone()], &com_hub).await;

        // get last block that was sent
        let mockup_interface_out = com_interface.clone();
        let mockup_interface_out = mockup_interface_out.borrow();

        assert!(mockup_interface_out.last_block().is_none());
    }
}

#[tokio::test]
pub async fn send_block_to_multiple_endpoints() {
    run_async! {
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
        com_hub.update_async().await;

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
    };
}

#[tokio::test]
pub async fn send_blocks_to_multiple_endpoints() {
    run_async! {
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
        com_hub.update_async().await;

        // send block to multiple receivers
        let _ = send_empty_block_and_update(
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
    };
}

#[tokio::test]
pub async fn default_interface_create_socket_first() {
    run_async! {
        init_global_context();
        let (com_hub, com_interface, _) =
            get_mock_setup_and_socket_for_priority(InterfacePriority::default())
                .await;

        let _ =
            send_empty_block_and_update(&[TEST_ENDPOINT_B.clone()], &com_hub)
                .await;

        let mockup_interface_out = com_interface.clone();
        let mockup_interface_out = mockup_interface_out.borrow();
        assert_eq!(mockup_interface_out.outgoing_queue.len(), 1);
    };
}

#[tokio::test]
pub async fn default_interface_set_default_interface_first() {
    run_async!({
        init_global_context();
        let (com_hub, com_interface) = get_mock_setup_with_endpoint(
            ORIGIN.clone(),
            InterfacePriority::default(),
        )
        .await;

        let socket = add_socket(com_interface.clone());
        register_socket_endpoint(
            com_interface.clone(),
            socket.clone(),
            TEST_ENDPOINT_A.clone(),
        );

        // Update to let the com_hub know about the socket and call the add_socket method
        // This will set the default interface and socket
        com_hub.update_async().await;
        let _ =
            send_empty_block_and_update(&[TEST_ENDPOINT_B.clone()], &com_hub)
                .await;

        let mockup_interface_out = com_interface.clone();
        let mockup_interface_out = mockup_interface_out.borrow();
        assert_eq!(mockup_interface_out.outgoing_queue.len(), 1);
    });
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
    run_async! {
        // init mock setup
        init_global_context();
        let (com_hub, _, socket) = get_mock_setup_and_socket().await;

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
        com_hub.update_async().await;

        let last_block = get_last_received_single_block_from_com_hub(&com_hub);
        assert_eq!(last_block.raw_bytes.clone().unwrap(), block_bytes);
    }
}

#[tokio::test]
pub async fn test_receive_multiple() {
    run_async! {
        // init mock setup
        init_global_context();
        let (com_hub, _, socket) = get_mock_setup_and_socket().await;

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

        com_hub.update_async().await;

        let incoming_blocks = get_all_received_single_blocks_from_com_hub(&com_hub);

        for (incoming_block, block) in incoming_blocks.iter().zip(blocks.iter()) {
            assert_eq!(
                incoming_block.raw_bytes.clone().unwrap(),
                block.to_bytes().unwrap()
            );
        }
    }
}

#[tokio::test]
pub async fn test_add_and_remove_interface_and_sockets() {
    run_async! {
        init_global_context();

        let (com_hub, com_interface, socket) = get_mock_setup_and_socket().await;

        assert_eq!(com_hub.interfaces.borrow().len(), 1);
        assert_eq!(com_hub.sockets.borrow().len(), 1);
        assert_eq!(com_hub.endpoint_sockets.borrow().len(), 1);

        assert_eq!(
            com_interface.borrow_mut().get_info().get_state(),
            ComInterfaceState::Connected
        );

        assert_eq!(socket.lock().unwrap().state, SocketState::Open);

        let uuid = com_interface.borrow().get_uuid().clone();

        // remove interface
        assert!(com_hub.remove_interface(uuid).await.is_ok());

        assert_eq!(com_hub.interfaces.borrow().len(), 0);
        assert_eq!(com_hub.sockets.borrow().len(), 0);
        assert_eq!(com_hub.endpoint_sockets.borrow().len(), 0);

        assert_eq!(
            com_interface.borrow_mut().get_info().get_state(),
            ComInterfaceState::Destroyed
        );

        assert_eq!(socket.lock().unwrap().state, SocketState::Destroyed);
    };
}

#[tokio::test]
pub async fn test_basic_routing() {
    run_async! {
        init_global_context();
        let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
        let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

        let (com_hub_mut_a, com_interface_a, socket_a) =
            get_mock_setup_and_socket_for_endpoint(
                TEST_ENDPOINT_A.clone(),
                None,
                Some(sender_a),
                Some(receiver_b),
                InterfacePriority::default(),
            )
            .await;

        let (com_hub_mut_b, com_interface_b, socket_b) =
            get_mock_setup_and_socket_for_endpoint(
                TEST_ENDPOINT_B.clone(),
                None,
                Some(sender_b),
                Some(receiver_a),
                InterfacePriority::default(),
            )
            .await;

        com_interface_a.borrow_mut().update();
        com_interface_b.borrow_mut().update();

        com_hub_mut_a.update_async().await;
        com_hub_mut_b.update_async().await;

        let block_a_to_b = send_block_with_body(
            &[TEST_ENDPOINT_B.clone()],
            b"Hello world",
            &com_hub_mut_a,
        )
        .await;

        com_interface_b.borrow_mut().update();
        com_hub_mut_b.update_async().await;

        let last_block =
            get_last_received_single_block_from_com_hub(&com_hub_mut_b);
        assert_eq!(block_a_to_b.body, last_block.body);
    };
}

#[tokio::test]
pub async fn register_factory() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();
            let mut com_hub = ComHub::default();
            MockupInterface::register_on_com_hub(&mut com_hub);

            assert_eq!(com_hub.interface_factories.borrow().len(), 1);
            assert!(com_hub
                .interface_factories
                .borrow()
                .get("mockup")
                .is_some());

            // create a new mockup interface from the com_hub
            let mockup_interface = com_hub
                .create_interface(
                    "mockup",
                    Box::new(MockupInterfaceSetupData::new("mockup")),
                    InterfacePriority::default(),
                )
                .await
                .unwrap();

            assert_eq!(
                mockup_interface
                    .borrow_mut()
                    .get_properties()
                    .interface_type,
                "mockup"
            );
        })
        .await;
}

#[tokio::test]
pub async fn test_reconnect() {
    init_global_context();
    let com_hub = ComHub::default();

    // create a new interface, open it and add it to the com_hub
    let mut base_interface =
        BaseInterface::new_with_properties(InterfaceProperties {
            reconnection_config: ReconnectionConfig::ReconnectWithTimeout {
                timeout: std::time::Duration::from_secs(1),
            },
            ..InterfaceProperties::default()
        });
    base_interface.open().unwrap();
    let base_interface = Rc::new(RefCell::new(base_interface));
    com_hub
        .add_interface(base_interface.clone(), InterfacePriority::default())
        .unwrap();

    // check that the interface is connected
    assert_eq!(
        base_interface.borrow().get_state(),
        ComInterfaceState::Connected
    );

    // check that the interface is in the com_hub
    assert_eq!(com_hub.interfaces.borrow().len(), 1);
    assert!(com_hub.has_interface(base_interface.borrow().get_uuid()));

    let com_hub = Rc::new(RefCell::new(com_hub));

    // simulate a disconnection by closing the interface
    // This action is normally done by the interface itself
    // but we do it manually here to test the reconnection
    assert!(base_interface.borrow_mut().close().await);

    // check that the interface is not connected
    // and that the close_timestamp is set
    assert_eq!(
        base_interface.borrow().get_state(),
        ComInterfaceState::NotConnected
    );

    assert!(base_interface
        .borrow_mut()
        .get_properties()
        .close_timestamp
        .is_some());

    // the interface should not be reconnected yet
    com_hub.borrow().update_async().await;
    assert_eq!(
        base_interface.borrow().get_state(),
        ComInterfaceState::NotConnected
    );

    // wait for the reconnection to happen
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // check that the interface is connected again
    // and that the close_timestamp is reset
    com_hub.borrow().update_async().await;
    assert_eq!(
        base_interface.borrow().get_state(),
        ComInterfaceState::Connected
    );
}
