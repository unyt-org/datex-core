use core::panic;
use std::net::SocketAddr;

use crate::context::init_global_context;
use datex_core::network::com_interfaces::com_interface::ComInterfaceState;
use datex_core::network::com_interfaces::{
    com_interface::ComInterface, socket_provider::MultipleSocketProvider,
    webrtc::webrtc_client_interface::WebRTCClientInterface,
};
use futures::{select, FutureExt};
use futures_timer::Delay;
use log::info;
use matchbox_signaling::SignalingServer;
use matchbox_socket::PeerState;
use matchbox_socket::WebRtcSocket;
use std::time::Duration;
use tokio::spawn;

pub fn start_server(url: &str) {
    let server =
        SignalingServer::full_mesh_builder(url.parse::<SocketAddr>().unwrap())
            .on_connection_request(|_| Ok(true))
            .on_id_assignment(|(socket, id)| info!("{socket} received {id}"))
            .on_peer_connected(|id| info!("Peer connected: {id}"))
            .on_peer_disconnected(|id| info!("Peer disconnected: {id}"))
            .cors()
            .trace()
            .build();

    spawn(async move {
        server.serve().await.unwrap_or_else(|e| {
            panic!("Failed to start signaling server: {:?}", e);
        });
    });
}

#[tokio::test]
pub async fn test_construct() {
    init_global_context();
    let client_a = WebRTCClientInterface::open_reliable(
        &format!("ws://invalid.interface:1234"),
        None,
    )
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });
    assert_eq!(client_a.get_state(), ComInterfaceState::Created);
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    assert_eq!(client_a.get_state(), ComInterfaceState::Connecting);
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    assert_eq!(client_a.get_state(), ComInterfaceState::Closed);
}

#[tokio::test]
pub async fn test_send_receive() {
    const PORT: u16 = 8081;
    const CLIENT_A_TO_CLIENT_B_MSG: &[u8] = b"Hello World";
    const CLIENT_B_TO_CLIENT_A_MSG: &[u8] = b"Nooo, this is Patrick!";
    let url = format!("127.0.0.1:{}", PORT);
    init_global_context();
    start_server(&url);

    let mut client_a = WebRTCClientInterface::open_reliable(
        &format!("ws://127.0.0.1:{}", PORT),
        None,
    )
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });

    let mut client_b = WebRTCClientInterface::open_reliable(
        &format!("ws://127.0.0.1:{}", PORT),
        None,
    )
    .await
    .unwrap_or_else(|e| {
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
}
