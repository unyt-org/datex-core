use crate::context::init_global_context;
use datex_core::network::com_interfaces::default_com_interfaces::websocket::websocket_common::WebSocketError;
use datex_core::network::com_interfaces::socket_provider::MultipleSocketProvider;
use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::{
        websocket::websocket_client_native_interface::WebSocketClientNativeInterface,
        websocket::websocket_server_native_interface::WebSocketServerNativeInterface,
    },
    socket_provider::SingleSocketProvider,
};

use std::{cell::RefCell, rc::Rc};

#[tokio::test]
pub async fn test_create_socket_connection() {
    const PORT: u16 = 8085;
    init_global_context();

    const CLIENT_TO_SERVER_MSG: &[u8] = b"Hello World";
    const SERVER_TO_CLIENT_MSG: &[u8] = b"Nooo, this is Patrick!";

    let mut server = WebSocketServerNativeInterface::new(PORT).unwrap();
    server.open().await.unwrap_or_else(|e| {
        panic!("Failed to create WebSocketServerInterface: {e}");
    });

    let client = Rc::new(RefCell::new(
        WebSocketClientNativeInterface::new(&format!("ws://localhost:{PORT}"))
            .unwrap(),
    ));
    client.borrow_mut().open().await.unwrap_or_else(|e| {
        panic!("Failed to create WebSocketClientInterface: {e}");
    });
    let server = Rc::new(RefCell::new(server));

    let client_uuid = client.borrow().get_socket_uuid().unwrap();
    assert!(
        client
            .borrow_mut()
            .send_block(CLIENT_TO_SERVER_MSG, client_uuid.clone())
            .await
    );

    let server_uuid = server.borrow().get_socket_uuid_at(0).unwrap();
    assert!(
        server
            .borrow_mut()
            .send_block(SERVER_TO_CLIENT_MSG, server_uuid.clone())
            .await
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    {
        let server = server.clone();
        let server = server.borrow_mut();
        let socket = server.get_socket_with_uuid(server_uuid.clone()).unwrap();
        let socket = socket.lock().unwrap();
        let mut queue = socket.receive_queue.lock().unwrap();
        assert_eq!(queue.drain(..).collect::<Vec<_>>(), CLIENT_TO_SERVER_MSG);
    }

    {
        let client = client.clone();
        let client = client.borrow_mut();
        let socket = client.get_socket().unwrap();
        let socket = socket.lock().unwrap();
        let mut queue = socket.receive_queue.lock().unwrap();
        assert_eq!(queue.drain(..).collect::<Vec<_>>(), SERVER_TO_CLIENT_MSG);
    }

    let client = &mut *client.borrow_mut();
    client.destroy_ref().await;

    let server = &mut *server.borrow_mut();
    server.destroy_ref().await;
}

#[tokio::test]
pub async fn test_construct_client() {
    init_global_context();

    // Test with a invalid URL
    assert_eq!(
        WebSocketClientNativeInterface::new("ftp://localhost:1234")
            .unwrap_err(),
        WebSocketError::InvalidURL
    );

    // We expect a connection error here, as the server can't be reached
    let mut client =
        WebSocketClientNativeInterface::new("ws://localhost.invalid:1234")
            .unwrap();

    assert_eq!(
        client.open().await.unwrap_err(),
        WebSocketError::ConnectionError
    );
}
