use std::{cell::RefCell, rc::Rc, time::Duration};

use datex_core::{
    network::com_interfaces::{
        com_interface::ComInterface,
        com_interface_socket::ComInterfaceSocketUUID,
        default_com_interfaces::webrtc::{
            webrtc_common_new::webrtc_trait::{
                WebRTCTrait, WebRTCTraitInternal,
            },
            webrtc_native_interface::WebRTCNativeInterface,
        },
        socket_provider::SingleSocketProvider,
    },
    run_async,
    task::{sleep, spawn_local},
    utils::uuid::UUID,
};
use ntest_timeout::timeout;

use crate::{
    context::init_global_context,
    network::helpers::mock_setup::{TEST_ENDPOINT_A, TEST_ENDPOINT_B},
};

#[tokio::test]
#[timeout(10000)]
pub async fn test_connect() {
    const BLOCK_A_TO_B: &[u8] = b"long message";
    const BLOCK_B_TO_A: &[u8] = b"test";
    run_async! {
        init_global_context();
        let mut interface_a = WebRTCNativeInterface::new(
            TEST_ENDPOINT_A.clone(),
        );
        interface_a.open().await.unwrap();


        let mut interface_b = WebRTCNativeInterface::new(
            TEST_ENDPOINT_B.clone(),
        );
        interface_b.open().await.unwrap();

        let interface_a = Rc::new(RefCell::new(interface_a));
        let interface_b = Rc::new(RefCell::new(interface_b));

        let interface_a_clone = interface_a.clone();
        let inteface_b_clone = interface_b.clone();

        interface_a.clone().borrow().set_on_ice_candidate(Box::new(move |candidate| {
            let interface_b = inteface_b_clone.clone();
            // info!("Candidate A: {:?}", candidate);
            spawn_local(async move {
                interface_b.clone().borrow().add_ice_candidate(candidate).await.unwrap();
            });
        }));

        interface_b.clone().borrow().set_on_ice_candidate(Box::new(move |candidate| {
            let interface_a = interface_a_clone.clone();
            // info!("Candidate B: {:?}", candidate);
            spawn_local(async move {
                interface_a.clone().borrow().add_ice_candidate(candidate).await.unwrap();
            });
        }));


        let offer = interface_a.clone().borrow().create_offer().await.unwrap();
        let answer = interface_b.clone().borrow().create_answer(offer).await.unwrap();
        interface_a.clone().borrow().set_answer(answer).await.unwrap();

        interface_a.borrow().wait_for_connection().await.unwrap();
        interface_b.borrow().wait_for_connection().await.unwrap();

        let socket_stub = ComInterfaceSocketUUID(UUID::from_string("uuid".to_string()));
        assert!(
            interface_a.clone().borrow_mut().send_block(BLOCK_A_TO_B, socket_stub.clone()).await
        );
        assert!(
            interface_b.clone().borrow_mut().send_block(BLOCK_B_TO_A, socket_stub.clone()).await
        );
        sleep(Duration::from_secs(1)).await;

        let receive_a = {
            let  socket = interface_a.borrow_mut().get_socket();
            let socket = socket.unwrap();
            let socket = socket.lock().unwrap();
            let mut socket = socket.receive_queue.lock().unwrap();
            socket.drain(..).collect::<Vec<_>>()
        };
        let receive_b = {
            let  socket = interface_b.borrow_mut().get_socket();
            let socket = socket.unwrap();
            let socket = socket.lock().unwrap();
            let mut socket = socket.receive_queue.lock().unwrap();
            socket.drain(..).collect::<Vec<_>>()
        };

        assert_eq!(receive_a, BLOCK_B_TO_A);
        assert_eq!(receive_b, BLOCK_A_TO_B);
    }
}
