

use datex_core::network::com_interfaces::websocket::websocket_client::WebSocketClientInterface;

use crate::context::init_global_context;

#[test]
pub fn test_construct() {
    init_global_context();
    let client = WebSocketClientInterface::new("ws://localhost:8080").unwrap();
}

// FIXME TODO
// #[tokio::test]
// pub async fn test_client_connect() {
//     init_global_context();

//     let server = &mut WebSocketServerInterface::new(8080).unwrap();
//     server.connect().unwrap();
//     sleep(Duration::from_secs(2)).await;
//     info!("Server connected");

//     let client =
//         &mut WebSocketClientInterface::new("ws://localhost:8080").unwrap();
//     client.connect().unwrap();
//     info!("Client connected");

//     sleep(Duration::from_secs(2)).await;
//     // ComInterfaceSocket::new(ComInterfaceUUID::, direction, channel_factor)
//     client.send_block(b"Hello", None);
//     info!("Message sent");
// }
