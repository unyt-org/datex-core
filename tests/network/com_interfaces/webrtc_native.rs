use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::webrtc::{
        webrtc_common::WebRTCInterfaceTrait,
        webrtc_native_interface::WebRTCNativeInterface,
    },
    socket_provider::SingleSocketProvider,
};
use ntest_timeout::timeout;

use crate::{
    context::init_global_context,
    network::helpers::mock_setup::{TEST_ENDPOINT_A, TEST_ENDPOINT_B},
};

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
