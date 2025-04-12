use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::{
        tcp_client_native::TCPClientNativeInterface,
        tcp_server_native::TCPServerNativeInterface,
    },
    socket_provider::SingleSocketProvider,
    webrtc::webrtc_client::WebRTCClientInterface,
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8081;
    const CLIENT_A_TO_CLIENT_B_MSG: &[u8] = b"Hello World";
    const CLIENT_B_TO_CLIENT_A_MSG: &[u8] = b"Nooo, this is Patrick!";

    init_global_context();

    let mut client_a = WebRTCClientInterface::open_reliable(&format!(
        "ws://localhost:{}",
        PORT
    ))
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });

    let mut client_b = WebRTCClientInterface::open_reliable(&format!(
        "ws://localhost:{}",
        PORT
    ))
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });
}
