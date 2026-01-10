use crate::network::helpers::mock_setup::{
    TEST_ENDPOINT_A, TEST_ENDPOINT_ORIGIN, get_mock_setup_and_socket,
};
use crate::network::helpers::mockup_interface::MockupInterface;
use core::cell::RefCell;
use datex_core::global::dxb_block::{DXBBlock, IncomingSection};
use datex_core::global::protocol_structures::block_header::{
    BlockHeader, BlockType, FlagsAndTimestamp,
};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::network::block_handler::IncomingSectionsSinkType;
use datex_core::run_async;
use datex_core::utils::context::init_global_context;
use log::info;
use std::rc::Rc;
use std::sync::mpsc;
use tokio::task::yield_now;

#[tokio::test]
async fn receive_single_block() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, _) = get_mock_setup_and_socket(IncomingSectionsSinkType::Collector).await;
        {
            let mut mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.receiver = Rc::new(RefCell::new(Some(receiver)));
        }

        let context_id = com_hub.block_handler.get_new_context_id();

        // Create a single DXB block
        let mut block = DXBBlock {
            block_header: BlockHeader {
                context_id,
                flags_and_timestamp: FlagsAndTimestamp::new()
                    .with_is_end_of_section(true)
                    .with_is_end_of_context(true),
                ..BlockHeader::default()
            },
            routing_header: RoutingHeader::default()
                .with_sender(TEST_ENDPOINT_A.clone())
                .to_owned(),
            ..DXBBlock::default()
        };
        block.set_receivers(vec![TEST_ENDPOINT_ORIGIN.clone()]);

        let block_bytes = block.to_bytes().unwrap();
        let block_endpoint_context_id = block.get_endpoint_context_id();

        // Put into incoming queue of mock interface
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }

        // wait a tick to allow processing
        yield_now().await;

        // block must be in incoming_sections_queue
        let sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(sections.len(), 1);
        let section = sections.first().unwrap();

        // block must be a single block
        match section {
            IncomingSection::SingleBlock((Some(block), ..)) => {
                info!("section: {section:?}");
                assert_eq!(block.get_endpoint_context_id(), block_endpoint_context_id);
            }
            _ => core::panic!("Expected a SingleBlock section"),
        }
    }
}

#[tokio::test]
async fn receive_multiple_blocks() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, _) = get_mock_setup_and_socket(IncomingSectionsSinkType::Collector).await;
        {
            let mut mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.receiver = Rc::new(RefCell::new(Some(receiver)));
        }
        let context_id = com_hub.block_handler.get_new_context_id();
        let section_index = 42;

        // Create a single DXB block
        let mut blocks = vec![
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index,
                    block_number: 0,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_context(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index,
                    block_number: 1,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_context(true),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
        ];

        // Set receiver for each block
        for block in &mut blocks {
            block.set_receivers(vec![TEST_ENDPOINT_ORIGIN.clone()]);
        }

        // 1. Send first block
        let block_bytes = blocks[0].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }

        // wait a tick to allow processing
        yield_now().await;

        // block must be in incoming_sections_queue
        let mut sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(sections.len(), 1);
        let section = sections.first_mut().unwrap();
        match section {
            IncomingSection::BlockStream((Some(blocks), incoming_context_section_id)) => {
                // section must match
                assert_eq!(incoming_context_section_id.section_index, section_index);
                // blocks queue must contain the first block
                assert!(section.next().await.is_some());
            }
            _ => core::panic!("Expected a BlockStream section"),
        }


        // 2. Send second block
        let block_bytes = blocks[1].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let  mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }

        // wait a tick to allow processing
        yield_now().await;

        // no new incoming sections, old section receives new blocks
        assert_eq!(com_hub.block_handler.drain_collected_sections().len(), 0);
        // block must be a block stream
        match &section {
            IncomingSection::BlockStream((Some(blocks), incoming_context_section_id)) => {
                // section must match
                assert_eq!(incoming_context_section_id.section_index, section_index);
                // blocks queue length must be 2 (was not yet drained)
                assert_eq!(section.drain().await.len(), 1);
            }
            _ => core::panic!("Expected a BlockStream section"),
        }
    }
}

#[tokio::test]
async fn receive_multiple_blocks_wrong_order() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket_uuid) = get_mock_setup_and_socket(IncomingSectionsSinkType::Collector).await;
        {
            let mut mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.receiver = Rc::new(RefCell::new(Some(receiver)));
        }

        let context_id = com_hub.block_handler.get_new_context_id();
        let section_index = 42;

        // Create a single DXB block
        let mut blocks = vec![
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index,
                    block_number: 1,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_context(true),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index,
                    block_number: 0,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_context(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
        ];

        // Set receiver for each block
        for block in &mut blocks {
            block.set_receivers(vec![TEST_ENDPOINT_ORIGIN.clone()]);
        }

        // 1. Send first block
        let block_bytes = blocks[0].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let mut mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }
        yield_now().await;

        // block is not in incoming_sections_queue
        let sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(sections.len(), 0);

        // 2. Send second block
        let block_bytes = blocks[1].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }
        yield_now().await;

        // block must be in incoming_sections_queue
        let mut sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(sections.len(), 1);

        let section = sections.first_mut().unwrap();
        // block must be a block stream
        match section {
            IncomingSection::BlockStream((Some(blocks), incoming_context_section_id)) => {
                // section must match
                assert_eq!(incoming_context_section_id.section_index.clone(), section_index);
                // blocks queue length must be 2
                let blocks = section.drain().await;
                assert_eq!(blocks.len(), 2);

                // check order:
                // first block must have block number 0
                let block = blocks.first().unwrap();
                assert_eq!(block.block_header.block_number, 0);
                // second block must have block number 1
                let block = blocks.get(1).unwrap();
                assert_eq!(block.block_header.block_number, 1);
            }
            _ => core::panic!("Expected a BlockStream section"),
        }
    }
}

#[tokio::test]
async fn receive_multiple_sections() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket_uuid) = get_mock_setup_and_socket(IncomingSectionsSinkType::Collector).await;
        {
            let mut mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.receiver = Rc::new(RefCell::new(Some(receiver)));
        }

        let context_id = com_hub.block_handler.get_new_context_id();
        let section_index_1 = 42;
        let section_index_2 = 43;

        // Create a single DXB block
        let mut blocks = vec![
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index: section_index_1,
                    block_number: 0,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_context(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index: section_index_1,
                    block_number: 1,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_context(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index: section_index_2,
                    block_number: 2,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(false)
                        .with_is_end_of_context(false),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
            DXBBlock {
                block_header: BlockHeader {
                    context_id,
                    section_index: section_index_2,
                    block_number: 3,
                    flags_and_timestamp: FlagsAndTimestamp::new()
                        .with_is_end_of_section(true)
                        .with_is_end_of_context(true),
                    ..BlockHeader::default()
                },
                routing_header: RoutingHeader::default()
                    .with_sender(TEST_ENDPOINT_A.clone())
                    .to_owned(),
                ..DXBBlock::default()
            },
        ];


        // Set receiver for each block
        for block in &mut blocks {
            block.set_receivers(vec![TEST_ENDPOINT_ORIGIN.clone()]);
        }

        // 1. Send first block
        let block_bytes = blocks[0].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let  mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }

        yield_now().await;

        // block must be in incoming_sections_queue
        let mut sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(sections.len(), 1);
        let section = sections.first_mut().unwrap();
        // block must be a block stream
        match section {
            IncomingSection::BlockStream((Some(blocks), incoming_context_section_id)) => {
                // section must match
                assert_eq!(incoming_context_section_id.section_index, section_index_1);
                // block queue must contain the first block
                assert!(section.next().await.is_some());
            }
            _ => core::panic!("Expected a BlockStream section"),
        }

        // 2. Send second block
        let block_bytes = blocks[1].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let  mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }
        yield_now().await;

        // block must not be in incoming_sections_queue
        let new_sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(new_sections.len(), 0);

        let section = sections.first_mut().unwrap();

        // block must be a block stream
        match section {
            IncomingSection::BlockStream((Some(blocks), incoming_context_section_id)) => {
                // section must match
                assert_eq!(incoming_context_section_id.section_index, section_index_1);
                // blocks queue length must be 1
                assert_eq!(section.drain().await.len(), 1);
            }
            _ => core::panic!("Expected a BlockStream section"),
        }

        // 3. Send third block
        let block_bytes = blocks[2].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let  mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }
        yield_now().await;

        // block must be in incoming_sections_queue
        let mut sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(sections.len(), 1);
        let section = sections.first_mut().unwrap();
        // block must be a block stream
        match section {
            IncomingSection::BlockStream((Some(blocks), incoming_context_section_id)) => {
                // section must match
                assert_eq!(incoming_context_section_id.section_index, section_index_2);
                // block queue must contain the first block
                assert!(section.next().await.is_some());
            }
            _ => core::panic!("Expected a BlockStream section"),
        }

        // 4. Send fourth block
        let block_bytes = blocks[3].to_bytes().unwrap();
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let  mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }
        yield_now().await;

        // block must not be in incoming_sections_queue
        let new_sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(new_sections.len(), 0);

        let section = sections.first_mut().unwrap();

        // block must be a block stream
        match section {
            IncomingSection::BlockStream((Some(blocks), incoming_context_section_id)) => {
                // section must match
                assert_eq!(incoming_context_section_id.section_index, section_index_2);
                // blocks queue length must be 1
                assert_eq!(section.drain().await.len(), 1);
            }
            _ => core::panic!("Expected a BlockStream section"),
        }
    }
}

#[tokio::test]
async fn await_response_block() {
    run_async! {
        init_global_context();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (com_hub, com_interface, socket_uuid) = get_mock_setup_and_socket(IncomingSectionsSinkType::Collector).await;
        {
            let mut mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.receiver = Rc::new(RefCell::new(Some(receiver)));
        }

        let context_id = com_hub.block_handler.get_new_context_id();
        let section_index = 42;

        // Create a single DXB block
        let mut block = DXBBlock {
            block_header: BlockHeader {
                context_id,
                section_index,
                flags_and_timestamp: FlagsAndTimestamp::new()
                    .with_block_type(BlockType::Response)
                    .with_is_end_of_section(true)
                    .with_is_end_of_context(true),
                ..BlockHeader::default()
            },
            routing_header: RoutingHeader::default().with_sender(TEST_ENDPOINT_A.clone()).to_owned(),
            ..DXBBlock::default()
        };
        block.set_receivers(vec![TEST_ENDPOINT_ORIGIN.clone()]);

        // set observer for the block
        let mut rx = com_hub.block_handler.register_incoming_block_observer(
            context_id,
            section_index,
        );

        let block_bytes = block.to_bytes().unwrap();

        // Put into incoming queue of mock interface
        sender.send(block_bytes).unwrap();
        // update the com interface
        {
            let mockup_interface_impl = com_interface
                .implementation_mut::<MockupInterface>();
            mockup_interface_impl.update().await;
        }
        yield_now().await;

        // block must not be in incoming_sections_queue
        let sections = com_hub.block_handler.drain_collected_sections();
        assert_eq!(sections.len(), 0);

        // await receiver
        let response = rx.next().await.unwrap();

        // IncomingSection must be a SingleBlock
        match response {
            IncomingSection::SingleBlock((Some(block), _)) => {
                info!("section: {block:?}");
                assert_eq!(block.block_header.context_id, context_id);
                assert_eq!(block.block_header.section_index, section_index);
            }
            _ => core::panic!("Expected a SingleBlock section"),
        }
    }
}
