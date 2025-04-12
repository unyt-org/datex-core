use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::tcp_client_native::TCPClientNativeInterface,
    socket_provider::SingleSocketProvider,
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8080;
    const CLIENT_TO_SERVER_MSG: &[u8] = b"Hello World";
    const SERVER_TO_CLIENT_MSG: &[u8] = b"Nooo, this is Patrick!";

    init_global_context();

    let mut client =
        TCPClientNativeInterface::open(&format!("ws://localhost:{}", PORT))
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create WebSocketClientInterface: {}", e);
            });

    assert!(
        client
            .send_block(CLIENT_TO_SERVER_MSG, client.get_socket_uuid().unwrap())
            .await
    );
}
