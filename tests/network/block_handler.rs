use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{
    get_mock_setup_and_socket, TEST_ENDPOINT_A, TEST_ENDPOINT_ORIGIN,
};
use datex_core::global::dxb_block::{DXBBlock, IncomingSection};
use datex_core::global::protocol_structures::block_header::{
    BlockHeader, BlockType, FlagsAndTimestamp,
};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::run_async;
use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

#[tokio::test]
async fn receive_single_block() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket) = get_mock_setup_and_socket().await;
        com_interface.borrow_mut().receiver = Rc::new(RefCell::new(Some(receiver)));

        let scope_id = com_hub.block_handler.get_new_scope_id();

        // Create a single DXB block
        let mut block = DXBBlock {
            block_header: BlockHeader {
                scope_id,
                flags_and_timestamp: FlagsAndTimestamp::new()
                    .with_is_end_of_section(true)
                    .with_is_end_of_scope(true),
                ..BlockHeader::default()
            },
            routing_header: RoutingHeader {
                sender: TEST_ENDPOINT_A.clone(),
                ..RoutingHeader::default()
            },
            ..DXBBlock::default()
        };
        block.set_receivers(&[TEST_ENDPOINT_ORIGIN.clone()]);

        let block_bytes = block.to_bytes().unwrap();
        let block_bytes_len = block_bytes.len();
        let block_endpoint_scope_id = block.get_endpoint_scope_id();

        // Put into incoming queue of mock interface
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // Check if the block was sent to the socket
        assert_eq!(socket.lock().unwrap().receive_queue.lock().unwrap().len(), block_bytes_len);

        // update the com hub
        com_hub.update_async().await;

        // block must be in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow().clone();
        assert_eq!(sections.len(), 1);
        let section = sections.iter().next().unwrap();

        // block must be a single block
        match section {
            IncomingSection::SingleBlock(block) => {
                info!("section: {section:?}");
                assert_eq!(block.get_endpoint_scope_id(), block_endpoint_scope_id);
            }
            _ => panic!("Expected a SingleBlock section"),
        }
    }
}

#[tokio::test]
async fn receive_multiple_blocks() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket) = get_mock_setup_and_socket().await;
        com_interface.borrow_mut().receiver = Rc::new(RefCell::new(Some(receiver)));

        let scope_id = com_hub.block_handler.get_new_scope_id();
        let section_index = 42;

        // Create a single DXB block
        let mut blocks = vec![
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index,
                    block_number: 0,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_scope(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index,
                    block_number: 1,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_scope(true),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
        ];

        // Set receiver for each block
        for block in &mut blocks {
            block.set_receivers(&[TEST_ENDPOINT_ORIGIN.clone()]);
        }

        // 1. Send first block
        let block_bytes = blocks[0].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;

        // block must be in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(sections.len(), 1);
        let section = sections.first().unwrap();
        // block must be a block stream
        match section {
            IncomingSection::BlockStream((blocks, incoming_section_index)) => {
                info!("section: {section:?}");
                // section must match
                assert_eq!(incoming_section_index, &section_index);
                // blocks queue length must be 1
                assert_eq!(blocks.borrow().len(), 1);
            }
            _ => panic!("Expected a BlockStream section"),
        }

        // 2. Send second block
        let block_bytes = blocks[1].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;

        // block must be in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow().clone();
        // no new incoming sections, old section receives new blocks
        assert_eq!(sections.len(), 0);
        // block must be a block stream
        match section {
            IncomingSection::BlockStream((blocks, incoming_section_index)) => {
                info!("section: {section:?}");
                // section must match
                assert_eq!(incoming_section_index, &section_index);
                // blocks queue length must be 2 (was not yet drained)
                assert_eq!(blocks.borrow().len(), 2);
            }
            _ => panic!("Expected a BlockStream section"),
        }
    }
}

#[tokio::test]
async fn receive_multiple_blocks_wrong_order() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket) = get_mock_setup_and_socket().await;
        com_interface.borrow_mut().receiver = Rc::new(RefCell::new(Some(receiver)));

        let scope_id = com_hub.block_handler.get_new_scope_id();
        let section_index = 42;

        // Create a single DXB block
        let mut blocks = vec![
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index,
                    block_number: 1,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_scope(true),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index,
                    block_number: 0,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_scope(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
        ];

        // Set receiver for each block
        for block in &mut blocks {
            block.set_receivers(&[TEST_ENDPOINT_ORIGIN.clone()]);
        }

        // 1. Send first block
        let block_bytes = blocks[0].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;

        // block is not in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(sections.len(), 0);

        // 2. Send second block
        let block_bytes = blocks[1].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;

        // block must be in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(sections.len(), 1);

        // block must be a block stream
        match sections.first().unwrap() {
            IncomingSection::BlockStream((blocks, incoming_section_index)) => {
                info!("section: {sections:?}");

                let blocks = blocks.borrow();
                // section must match
                assert_eq!(incoming_section_index, &section_index);
                // blocks queue length must be 2
                assert_eq!(blocks.len(), 2);

                // check order:
                // first block must have block number 0
                let block = blocks.front().unwrap();
                assert_eq!(block.block_header.block_number, 0);
                // second block must have block number 1
                let block = blocks.get(1).unwrap();
                assert_eq!(block.block_header.block_number, 1);
            }
            _ => panic!("Expected a BlockStream section"),
        }
    }
}

#[tokio::test]
async fn receive_multiple_sections() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket) = get_mock_setup_and_socket().await;
        com_interface.borrow_mut().receiver = Rc::new(RefCell::new(Some(receiver)));

        let scope_id = com_hub.block_handler.get_new_scope_id();
        let section_index_1 = 42;
        let section_index_2 = 43;

        // Create a single DXB block
        let mut blocks = vec![
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index: section_index_1,
                    block_number: 0,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_scope(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index: section_index_1,
                    block_number: 1,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_scope(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index: section_index_2,
                    block_number: 2,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_scope(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    scope_id,
                    section_index: section_index_2,
                    block_number: 3,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_scope(true),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader {
                    sender: TEST_ENDPOINT_A.clone(),
                    ..RoutingHeader::default()
                },
                ..DXBBlock::default()
            },
        ];


        // Set receiver for each block
        for block in &mut blocks {
            block.set_receivers(&[TEST_ENDPOINT_ORIGIN.clone()]);
        }

        // 1. Send first block
        let block_bytes = blocks[0].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;
        // block must be in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(sections.len(), 1);
        // block must be a block stream
        match sections.first().unwrap() {
            IncomingSection::BlockStream((blocks, incoming_section_index)) => {
                info!("section: {sections:?}");
                // section must match
                assert_eq!(incoming_section_index, &section_index_1);
                // blocks queue length must be 1
                assert_eq!(blocks.borrow().len(), 1);
            }
            _ => panic!("Expected a BlockStream section"),
        }

        // 2. Send second block
        let block_bytes = blocks[1].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;

        // block must not be in incoming_sections_queue
        let new_sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(new_sections.len(), 0);
        // block must be a block stream
        match sections.first().unwrap() {
            IncomingSection::BlockStream((blocks, incoming_section_index)) => {
                info!("section: {sections:?}");
                // section must match
                assert_eq!(incoming_section_index, &section_index_1);
                // blocks queue length must be 2
                assert_eq!(blocks.borrow().len(), 2);
            }
            _ => panic!("Expected a BlockStream section"),
        }

        // 3. Send third block
        let block_bytes = blocks[2].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;
        // block must be in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(sections.len(), 1);
        // block must be a block stream
        match sections.first().unwrap() {
            IncomingSection::BlockStream((blocks, incoming_section_index)) => {
                info!("section: {sections:?}");
                // section must match
                assert_eq!(incoming_section_index, &section_index_2);
                // blocks queue length must be 1
                assert_eq!(blocks.borrow().len(), 1);
            }
            _ => panic!("Expected a BlockStream section"),
        }

        // 4. Send fourth block
        let block_bytes = blocks[3].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;
        // block must not be in incoming_sections_queue
        let new_sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(new_sections.len(), 0);
        // block must be a block stream
        match sections.first().unwrap() {
            IncomingSection::BlockStream((blocks, incoming_section_index)) => {
                info!("section: {sections:?}");
                // section must match
                assert_eq!(incoming_section_index, &section_index_2);
                // blocks queue length must be 2
                assert_eq!(blocks.borrow().len(), 2);
            }
            _ => panic!("Expected a BlockStream section"),
        }
    }
}

#[tokio::test]
async fn await_response_block() {
    run_async! {
           init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket) = get_mock_setup_and_socket().await;
        com_interface.borrow_mut().receiver = Rc::new(RefCell::new(Some(receiver)));

        let scope_id = com_hub.block_handler.get_new_scope_id();
        let section_index = 42;

        // Create a single DXB block
        let mut block = DXBBlock {
            block_header: BlockHeader {
                scope_id,
                section_index,
                flags_and_timestamp: FlagsAndTimestamp::new()
                    .with_block_type(BlockType::Response)
                    .with_is_end_of_section(true)
                    .with_is_end_of_scope(true),
                ..BlockHeader::default()
            },
            routing_header: RoutingHeader {
                sender: TEST_ENDPOINT_A.clone(),
                ..RoutingHeader::default()
            },
            ..DXBBlock::default()
        };
        block.set_receivers(&[TEST_ENDPOINT_ORIGIN.clone()]);

        // set observer for the block
        let rx = com_hub.block_handler.register_incoming_block_observer(
            scope_id,
            section_index,
        );

        let block_bytes = block.to_bytes().unwrap();

        // Put into incoming queue of mock interface
        sender.send(block_bytes).unwrap();
        // update the com interface
        com_interface.borrow_mut().update();
        // update the com hub
        com_hub.update_async().await;

        // block must not be in incoming_sections_queue
        let sections = com_hub.block_handler.incoming_sections_queue.borrow_mut().drain(..).collect::<Vec<_>>();
        assert_eq!(sections.len(), 0);

        // await receiver
        let response = rx.await.unwrap();

        // IncomingSection must be a SingleBlock
        match response {
            IncomingSection::SingleBlock(block) => {
                info!("section: {block:?}");
                assert_eq!(block.block_header.scope_id, scope_id);
                assert_eq!(block.block_header.section_index, section_index);
            }
            _ => panic!("Expected a SingleBlock section"),
        }

    }
}
