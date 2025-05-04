use datex_core::network::com_interfaces::default_com_interfaces::webrtc::webrtc_new_client_interface::WebRTCNewClientInterface;
use log::info;
use ntest_timeout::timeout;
use webrtc::media::audio::buffer::info;

use crate::context::init_global_context;

#[tokio::test]
// #[timeout(2000)]
pub async fn test_send_receive() {
    init_global_context();
    let mut interface_a = WebRTCNewClientInterface::new("a");
    interface_a.open().await.unwrap();

    let mut interface_b = WebRTCNewClientInterface::new("b");
    interface_b.open().await.unwrap();

    let session_request_a_to_b = interface_a.create_offer("@b").await;
    info!("Session request: {:?}", session_request_a_to_b);

    let session_request_b_to_a = interface_b.create_offer("@a").await;
    info!("Session request: {:?}", session_request_b_to_a);

    interface_a
        .set_offer("@b", session_request_b_to_a)
        .await
        .unwrap();
    // interface_b
    //     .set_offer("@a", session_request_a_to_b)
    //     .await
    //     .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
}
