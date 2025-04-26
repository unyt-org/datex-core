use crate::context::init_global_context;
use datex_core::datex_values::Endpoint;
use datex_core::global::dxb_block::DXBBlock;
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::{
        websocket::websocket_client_native_interface::WebSocketClientNativeInterface,
        websocket::websocket_server_native_interface::WebSocketServerNativeInterface,
    },
    socket_provider::SingleSocketProvider,
};
use datex_core::runtime::Runtime;
use std::cell::RefCell;
use std::rc::Rc;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8080;
    const CLIENT_TO_SERVER_MSG: &[u8] = b"Hello World";
    const SERVER_TO_CLIENT_MSG: &[u8] = b"Nooo, this is Patrick!";

    init_global_context();

    let mut server = WebSocketServerNativeInterface::new(PORT)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to create WebSocketServerInterface: {e}");
        });

    let mut client =
        WebSocketClientNativeInterface::new(&format!("ws://localhost:{PORT}"))
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create WebSocketClientInterface: {e}");
            });

    assert!(
        client
            .send_block(CLIENT_TO_SERVER_MSG, client.get_socket_uuid().unwrap())
            .await
    );

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
}

#[tokio::test]
pub async fn test_create_socket_connection() {
    const PORT: u16 = 8085;

    let block = DXBBlock {
        routing_header: RoutingHeader {
            sender: Endpoint::from_string("@test").unwrap(),
            ..RoutingHeader::default()
        },
        ..DXBBlock::default()
    };

    let runtime = Runtime::init_native(Endpoint::default());

    let server = Rc::new(RefCell::new(
        WebSocketServerNativeInterface::new(PORT)
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create WebSocketServerInterface: {e}");
            }),
    ));

    let client = Rc::new(RefCell::new(
        WebSocketClientNativeInterface::new(&format!("ws://localhost:{PORT}"))
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create WebSocketClientInterface: {e}");
            }),
    ));

    runtime
        .com_hub
        .lock()
        .unwrap()
        .add_interface(server.clone())
        .unwrap();
    runtime
        .com_hub
        .lock()
        .unwrap()
        .add_interface(client.clone())
        .unwrap();

    let client_uuid = client.borrow().get_socket_uuid().unwrap();
    assert!(
        client
            .borrow_mut()
            .send_block(&block.to_bytes().unwrap(), client_uuid)
            .await
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    runtime.com_hub.lock().unwrap().update().await;

    // TODO: assert that endpoint socket was registered
}

#[ignore]
#[tokio::test]
pub async fn test_construct_client() {
    const PORT: u16 = 1234;

    let block = DXBBlock {
        routing_header: RoutingHeader {
            sender: Endpoint::from_string("@test").unwrap(),
            ..RoutingHeader::default()
        },
        ..DXBBlock::default()
    };

    init_global_context();

    let mut client =
        WebSocketClientNativeInterface::new(&format!("ws://localhost:{PORT}"))
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create WebSocketClientInterface: {e}");
            });

    assert!(
        client
            .send_block(
                &block.to_bytes().unwrap(),
                client.get_socket_uuid().unwrap()
            )
            .await
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;
}
