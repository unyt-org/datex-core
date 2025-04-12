use core::panic;
use std::net::SocketAddr;

use datex_core::network::com_interfaces::{
    com_interface::ComInterface,
    socket_provider::MultipleSocketProvider,
    webrtc::webrtc_client::WebRTCClientInterface,
};
use log::info;
use matchbox_signaling::SignalingServer;
use tokio::spawn;

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8081;
    const CLIENT_A_TO_CLIENT_B_MSG: &[u8] = b"Hello World";
    const CLIENT_B_TO_CLIENT_A_MSG: &[u8] = b"Nooo, this is Patrick!";
    let url = format!("127.0.0.1:{}", PORT);

    init_global_context();
    let server = SignalingServer::client_server_builder(
        url.parse::<SocketAddr>().unwrap(),
    )
    .on_connection_request(|connection| {
        info!("Connecting: {connection:?}");
        Ok(true)
    })
    .on_id_assignment(|(socket, id)| info!("{socket} received {id}"))
    .on_host_connected(|id| info!("Host joined: {id}"))
    .on_host_disconnected(|id| info!("Host left: {id}"))
    .on_client_connected(|id| info!("Client joined: {id}"))
    .on_client_disconnected(|id| info!("Client left: {id}"))
    .cors()
    .trace()
    .build();

    spawn(async move {
        server.serve().await.unwrap_or_else(|e| {
            panic!("Failed to start signaling server: {:?}", e);
        });
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let mut client_a = WebRTCClientInterface::open_reliable(&format!(
        "ws://localhost:{}",
        PORT
    ))
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });

    info!("client_a created");
    let client_b = WebRTCClientInterface::open_reliable(&format!(
        "ws://localhost:{}",
        PORT
    ))
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });
    info!("client_b created");

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // FIXME lock active here
    // assert_eq!(client_a.get_sockets_count(), 1);
    // assert_eq!(client_b.get_sockets_count(), 1);
    // panic!("B");

    let uuid = client_a.get_socket_uuid_at(0).unwrap();

    client_a.send_block(CLIENT_A_TO_CLIENT_B_MSG, uuid).await;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
}
