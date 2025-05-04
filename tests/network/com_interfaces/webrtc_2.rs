use datex_core::network::com_interfaces::default_com_interfaces::webrtc::webrtc_new_client_interface::WebRTCNewClientInterface;
use log::info;
use ntest_timeout::timeout;
use webrtc::media::audio::buffer::info;

use crate::context::init_global_context;

#[tokio::test]
// #[timeout(2000)]
pub async fn test_send_receive() {
    init_global_context();
    let mut interface = WebRTCNewClientInterface::new("test");
    interface.open().await.unwrap();

    let session_request = interface.create_offer().await;
    info!("Session request: {:?}", session_request);

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
}
