use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::tcp::{
        tcp_client_native_interface::TCPClientNativeInterface,
        tcp_common::TCPError,
        tcp_server_native_interface::TCPServerNativeInterface,
    },
    socket_provider::SingleSocketProvider,
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_client_no_connection() {
    init_global_context();

    let mut client =
        TCPClientNativeInterface::new("ws://localhost.invalid:8080")
            .unwrap();
    assert!(client.get_state().is_not_connected());
    let res = client.open().await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), TCPError::ConnectionError);
    assert!(client.get_state().is_not_connected());
    client.destroy().await;
}

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8081;
    const CLIENT_TO_SERVER_MSG: &[u8] = b"Hello World";
    const SERVER_TO_CLIENT_MSG: &[u8] = b"Nooo, this is Patrick!";

    init_global_context();

    let mut server = TCPServerNativeInterface::new(PORT).unwrap();
    server.open().await.unwrap_or_else(|e| {
        panic!("Failed to create TCPServerInterface: {e:?}");
    });

    let mut client =
        TCPClientNativeInterface::new(&format!("ws://localhost:{PORT}"))
            .unwrap();
    client.open().await.unwrap_or_else(|e| {
        panic!("Failed to create WebSocketClientInterface: {e}");
    });

    assert!(
        client
            .send_block(CLIENT_TO_SERVER_MSG, client.get_socket_uuid().unwrap())
            .await
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    let server_sockets = server.get_sockets().clone();
    let server_socket = server_sockets.lock().unwrap();
    let server_socket = server_socket.sockets.values().next().unwrap().clone();
    let uuid = {
        let server_socket = server_socket.lock().unwrap();
        server_socket.uuid.clone()
    };
    assert!(server.send_block(SERVER_TO_CLIENT_MSG, uuid).await);
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    // Check if the client received the message
    assert_eq!(
        client
            .get_socket()
            .unwrap()
            .lock()
            .unwrap()
            .receive_queue
            .lock()
            .unwrap()
            .drain(..)
            .collect::<Vec<_>>(),
        SERVER_TO_CLIENT_MSG
    );

    // Check if the server received the message
    assert_eq!(
        server_socket
            .lock()
            .unwrap()
            .receive_queue
            .lock()
            .unwrap()
            .drain(..)
            .collect::<Vec<_>>(),
        CLIENT_TO_SERVER_MSG
    );

    server.destroy().await;
    client.destroy().await;
}
