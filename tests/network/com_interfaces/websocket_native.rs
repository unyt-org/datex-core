use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::{
        websocket_client_native::WebSocketClientNativeInterface,
        websocket_server_native::WebSocketServerNativeInterface,
    },
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8080;
    init_global_context();

    let mut server = WebSocketServerNativeInterface::open(&PORT)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to create WebSocketServerInterface: {}", e);
        });

    let mut client = WebSocketClientNativeInterface::open(&format!(
        "ws://localhost:{}",
        PORT
    ))
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebSocketClientInterface: {}", e);
    });

    assert!(
        client
            .send_block(b"Hello", client.get_socket_uuid().unwrap())
            .await
    );

    let uuid = server
        .get_sockets()
        .lock()
        .unwrap()
        .sockets
        .values()
        .next()
        .unwrap()
        .lock()
        .unwrap()
        .uuid
        .clone();

    assert!(server.send_block(b"Hi", uuid).await);
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
