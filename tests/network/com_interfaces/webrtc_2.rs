use std::{cell::RefCell, rc::Rc};

use datex_core::network::com_interfaces::default_com_interfaces::webrtc::webrtc_new_client_interface::WebRTCNewClientInterface;
use log::info;
use ntest_timeout::timeout;
use webrtc::{ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit}, media::audio::buffer::info};

use crate::context::init_global_context;

#[tokio::test]
// #[timeout(2000)]
pub async fn test_send_receive() {
    init_global_context();
    let mut interface_a = WebRTCNewClientInterface::new("a");
    interface_a.open().await.unwrap();

    let mut interface_b = WebRTCNewClientInterface::new("b");
    interface_b.open().await.unwrap();

    let offer = interface_a.create_offer().await;
    interface_b.set_remote_description(offer).await;

    let answer = interface_b.create_answer().await;
    interface_a.set_remote_description(answer).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    for _ in 0..10 {
        for candidate in interface_a.ice_candidates.lock().unwrap().drain(..) {
            info!("Candidate A: {:?}", candidate);
            interface_b.add_ice_candidate(candidate).await;
        }
        for candidate in interface_b.ice_candidates.lock().unwrap().drain(..) {
            info!("Candidate B: {:?}", candidate);
            interface_a.add_ice_candidate(candidate).await;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // let session_request_a_to_b = interface_a.create_offer("@b").await;
    // info!("Session request: {:?}", session_request_a_to_b);

    // let session_request_b_to_a = interface_b.create_offer("@a").await;
    // info!("Session request: {:?}", session_request_b_to_a);

    // interface_a
    //     .set_offer("@b", session_request_b_to_a)
    //     .await
    //     .unwrap();
    // interface_b
    //     .set_offer("@a", session_request_a_to_b)
    //     .await
    //     .unwrap();
}
