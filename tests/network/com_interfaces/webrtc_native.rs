use std::{
    cell::RefCell,
    rc::Rc,
    time::Duration,
};

use datex_core::{
    network::com_interfaces::{
        com_interface::ComInterface,
        com_interface_socket::ComInterfaceSocketUUID,
        default_com_interfaces::webrtc::{
            webrtc_common::WebRTCInterfaceTrait,
            webrtc_common_new::webrtc_trait::{
                WebRTCTrait, WebRTCTraitInternal,
            },
            webrtc_native_interface,
            webrtc_native_interface_old::WebRTCNativeInterface,
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
#[timeout(8000)]
pub async fn test_new() {
    run_async! {
        init_global_context();
        let mut interface_a = webrtc_native_interface::WebRTCNativeInterface::new(
            TEST_ENDPOINT_A.clone(),
        );
        interface_a.open().await.unwrap();


        let mut interface_b = webrtc_native_interface::WebRTCNativeInterface::new(
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
        sleep(Duration::from_secs(1)).await;
        let answer = interface_b.clone().borrow().create_answer(offer).await.unwrap();
        sleep(Duration::from_secs(1)).await;
        interface_a.clone().borrow().set_answer(answer).await.unwrap();
        sleep(Duration::from_secs(2)).await;

        // let uuid = interface_b.clone().borrow().get_socket_uuid().clone().unwrap();
        interface_a.clone().borrow_mut().send_block(b"test", ComInterfaceSocketUUID(UUID::from_string("uuid".to_string()))).await;
    }
}

#[tokio::test]
#[timeout(5000)]
pub async fn test_send_receive() {
    init_global_context();
    let mut interface_a = WebRTCNativeInterface::new(TEST_ENDPOINT_B.clone());
    interface_a.open().await.unwrap();

    let mut interface_b = WebRTCNativeInterface::new(TEST_ENDPOINT_A.clone());
    interface_b.open().await.unwrap();

    let offer = interface_a.create_offer(true).await;
    interface_b.set_remote_description(offer).await.unwrap();

    let answer = interface_b.create_answer().await;
    interface_a.set_remote_description(answer).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    for _ in 0..2 {
        for candidate in interface_a.ice_candidates.lock().unwrap().drain(..) {
            // info!("Candidate A: {:?}", candidate);
            interface_b.add_ice_candidate(candidate).await.unwrap();
        }
        for candidate in interface_b.ice_candidates.lock().unwrap().drain(..) {
            // info!("Candidate B: {:?}", candidate);
            interface_a.add_ice_candidate(candidate).await.unwrap();
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let socket_a = interface_a.get_socket_uuid().unwrap();
    assert!(
        interface_a.send_block(b"Hello from A", socket_a).await,
        "Failed to send message from A"
    );

    let socket_b = interface_b.get_socket_uuid().unwrap();
    assert!(
        interface_b.send_block(b"Hello from B", socket_b).await,
        "Failed to send message from B"
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}
