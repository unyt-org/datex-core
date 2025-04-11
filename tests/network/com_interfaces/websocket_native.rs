use std::clone;

use datex_core::network::com_interfaces::websocket::{
    websocket_client::{WebSocket, WebSocketClientInterface},
    websocket_server::WebSocketServerInterface,
};
use tokio::sync::watch::error;

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8080;
    init_global_context();

    let server =
        WebSocketServerInterface::start(PORT)
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create WebSocketServerInterface: {}", e);
            });

    let client =
        WebSocketClientInterface::start(&format!("ws://localhost:{}", PORT))
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create WebSocketClientInterface: {}", e);
            });

    client
        .web_socket
        .clone()
        .borrow_mut()
        .send_block(b"Hello")
        .await;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
