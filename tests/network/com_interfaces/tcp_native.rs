use std::sync::{Arc, Mutex};

use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::tcp::{
        tcp_client_native_interface::TCPClientNativeInterface,
        tcp_common::TCPError,
        tcp_server_native_interface::TCPServerNativeInterface,
    },
    socket_provider::{MultipleSocketProvider, SingleSocketProvider},
};
use futures::future::join_all;
use datex_core::run_async;
use crate::context::init_global_context;

#[tokio::test]
pub async fn test_client_no_connection() {
    init_global_context();

    let mut client =
        TCPClientNativeInterface::new("0.0.0.0:8080").unwrap();
    assert!(client.get_state().is_not_connected());
    let res = client.open().await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), TCPError::ConnectionError);
    assert!(client.get_state().is_not_connected());
    client.destroy().await;
}

#[tokio::test]
pub async fn test_construct() {
    run_async! {
        const PORT: u16 = 8088;
        const CLIENT_TO_SERVER_MSG: &[u8] = b"Hello World";
        const SERVER_TO_CLIENT_MSG: &[u8] = b"Nooo, this is Patrick!";

        init_global_context();

        let mut server = TCPServerNativeInterface::new(PORT).unwrap();
        server.open().await.unwrap_or_else(|e| {
            core::panic!("Failed to create TCPServerInterface: {e:?}");
        });

        let mut client =
            TCPClientNativeInterface::new(&format!("0.0.0.0:{PORT}"))
                .unwrap();
        client.open().await.unwrap_or_else(|e| {
            core::panic!("Failed to create WebSocketClientInterface: {e}");
        });
        let client_uuid = client.get_socket_uuid().unwrap();

        assert!(
            client
                .send_block(CLIENT_TO_SERVER_MSG, client_uuid.clone())
                .await
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        let server_uuid = server.get_socket_uuid_at(0).unwrap();
        assert!(
            server
                .send_block(SERVER_TO_CLIENT_MSG, server_uuid.clone())
                .await
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        // Check if the client received the message
        assert_eq!(
            client
                .get_socket()
                .unwrap()
                .try_lock()
                .unwrap()
                .receive_queue
                .try_lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<_>>(),
            SERVER_TO_CLIENT_MSG
        );

        {
            // Check if the server received the message
            let server_socket = server.get_socket_with_uuid(server_uuid).unwrap();
            assert_eq!(
                server_socket
                    .try_lock()
                    .unwrap()
                    .receive_queue
                    .try_lock()
                    .unwrap()
                    .drain(..)
                    .collect::<Vec<_>>(),
                CLIENT_TO_SERVER_MSG
            );
        }

        // Parallel sending
        let client = Arc::new(Mutex::new(client));
        let mut futures = vec![];
        for _ in 0..5 {
            let client = client.clone();
            let client_uuid = client_uuid.clone();
            futures.push(async move {
                client
                    .try_lock()
                    .unwrap()
                    .send_block(CLIENT_TO_SERVER_MSG, client_uuid.clone())
                    .await;
            });
        }
        join_all(futures).await;

        // We take ownership of the client
        let client = Arc::into_inner(client).unwrap();
        let client = Mutex::into_inner(client).unwrap();
        client.destroy().await;

        server.destroy().await;
    }
}
