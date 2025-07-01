use core::panic;

use crate::context::init_global_context;
use crate::network::helpers::webrtc_signaling_server::start_server;
use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    default_com_interfaces::webrtc::matchbox_client_interface::MatchboxClientInterface,
    socket_provider::MultipleSocketProvider,
};

#[tokio::test]
pub async fn test_construct() {
    init_global_context();
    let mut client =
        MatchboxClientInterface::new_reliable("ws://interface.invalid", None)
            .unwrap();
    let result = client.open().await;
    assert!(result.is_err(), "Connection should fail");
}

#[tokio::test]
pub async fn test_send_receive() {
    const PORT: u16 = 8087;
    const CLIENT_A_TO_CLIENT_B_MSG: &[u8] = b"Hello World";
    const CLIENT_B_TO_CLIENT_A_MSG: &[u8] = b"Nooo, this is Patrick!";
    let url = format!("127.0.0.1:{PORT}");
    init_global_context();
    start_server(&url);

    let mut client_a = MatchboxClientInterface::new_reliable(
        &format!("ws://127.0.0.1:{PORT}"),
        None,
    )
    .unwrap();

    client_a.open().await.unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });

    let mut client_b = MatchboxClientInterface::new_reliable(
        &format!("ws://127.0.0.1:{PORT}"),
        None,
    )
    .unwrap();
    client_b.open().await.unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    assert_eq!(client_a.get_sockets_count(), 1);
    assert_eq!(client_b.get_sockets_count(), 1);

    let uuid_a_to_b = client_a.get_socket_uuid_at(0).unwrap();
    client_a
        .send_block(CLIENT_A_TO_CLIENT_B_MSG, uuid_a_to_b)
        .await;

    let uuid_b_to_a = client_b.get_socket_uuid_at(0).unwrap();
    client_b
        .send_block(CLIENT_B_TO_CLIENT_A_MSG, uuid_b_to_a)
        .await;

    let client_a_socket = client_a.get_socket_at(0).unwrap();
    let client_b_socket = client_b.get_socket_at(0).unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    assert_eq!(
        CLIENT_B_TO_CLIENT_A_MSG,
        client_a_socket
            .lock()
            .unwrap()
            .receive_queue
            .lock()
            .unwrap()
            .drain(..)
            .collect::<Vec<_>>()
    );

    assert_eq!(
        CLIENT_A_TO_CLIENT_B_MSG,
        client_b_socket
            .lock()
            .unwrap()
            .receive_queue
            .lock()
            .unwrap()
            .drain(..)
            .collect::<Vec<_>>()
    );

    client_a.destroy().await;
    client_b.destroy().await;
}
