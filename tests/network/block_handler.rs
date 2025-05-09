use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use datex_core::global::dxb_block::{DXBBlock, IncomingEndpointScopeId};
use datex_core::global::protocol_structures::block_header::{BlockHeader, FlagsAndTimestamp};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::run_async;
use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{get_mock_setup_and_socket, TEST_ENDPOINT_A, TEST_ENDPOINT_ORIGIN};

#[tokio::test]
async fn send_single_block() {
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
                    .with_is_end_of_section(true),
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

        // block scope id must be in request_queue
        assert_eq!(
            com_hub.block_handler.incoming_sections_queue.borrow().iter().next().unwrap().0,
            block_endpoint_scope_id
        );
        
        // block must be in request_scopes
        /*let scopes = com_hub.block_handler.block_cache.borrow();
        assert_eq!(
            scopes.len(),
            1,
        );*/
        
      /*  let scope_blocks = &scopes.get(&block_endpoint_scope_id).unwrap().block_queues;
        
        assert_eq!(
            scope_blocks.len(),
            1,
        );
        
        scope_blocks*/
    }
}