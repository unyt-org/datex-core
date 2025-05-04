use datex_core::network::com_interfaces::default_com_interfaces::webrtc::webrtc_new_client_interface::WebRTCNewClientInterface;

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_send_receive() {
    init_global_context();
    let mut interface = WebRTCNewClientInterface::new("test");
    interface.open().await.unwrap();
}
