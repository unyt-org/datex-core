use core::panic;
use std::net::SocketAddr;

use crate::context::init_global_context;
use datex_core::network::com_interfaces::{
    com_interface::ComInterface, socket_provider::MultipleSocketProvider,
    webrtc::webrtc_client::WebRTCClientInterface,
};
use futures::{select, FutureExt};
use futures_timer::Delay;
use log::info;
use matchbox_signaling::SignalingServer;
use matchbox_socket::PeerState;
use matchbox_socket::WebRtcSocket;
use std::time::Duration;
use tokio::spawn;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8081;
    const CLIENT_A_TO_CLIENT_B_MSG: &[u8] = b"Hello World";
    const CLIENT_B_TO_CLIENT_A_MSG: &[u8] = b"Nooo, this is Patrick!";
    let url = format!("127.0.0.1:{}", PORT);

    init_global_context();
    info!("Starting signaling server on {}", url);
    let server =
        SignalingServer::full_mesh_builder(url.parse::<SocketAddr>().unwrap())
            .on_connection_request(|_| Ok(true))
            .on_id_assignment(|(socket, id)| info!("{socket} received {id}"))
            .on_peer_connected(|id| info!("Peer connected: {id}"))
            .on_peer_disconnected(|id| info!("Peer disconnected: {id}"))
            // .on_host_connected(|id| info!("Host joined: {id}"))
            // .on_host_disconnected(|id| info!("Host left: {id}"))
            // .on_client_connected(|id| info!("Client joined: {id}"))
            // .on_client_disconnected(|id| info!("Client left: {id}"))
            .cors()
            .trace()
            .build();
    // let server =
    //     SignalingServer::full_mesh_builder(url.parse::<SocketAddr>().unwrap())
    //         .build();

    spawn(async move {
        server.serve().await.unwrap_or_else(|e| {
            panic!("Failed to start signaling server: {:?}", e);
        });
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let (mut socket1, loop_fut1) =
        WebRtcSocket::new_reliable("ws://localhost:8081/");

    spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let (mut socket2, loop_fut1) =
            WebRtcSocket::new_reliable("ws://localhost:8081/");
        loop_fut1.await.unwrap_or_else(|e| {
            panic!("Failed to start signaling server: {:?}", e);
        });
    });

    let loop_fut1 = loop_fut1.fuse();
    futures::pin_mut!(loop_fut1);

    let timeout = Delay::new(Duration::from_millis(100));
    futures::pin_mut!(timeout);

    loop {
        // Handle any new peers
        for (peer, state) in socket1.update_peers() {
            match state {
                PeerState::Connected => {
                    info!("Peer joined123: {peer}");
                    let packet =
                        "hello friend!".as_bytes().to_vec().into_boxed_slice();
                    socket1.channel_mut(0).send(packet, peer);
                }
                PeerState::Disconnected => {
                    info!("Peer left: {peer}");
                }
            }
        }

        // Accept any messages incoming
        for (peer, packet) in socket1.channel_mut(0).receive() {
            let message = String::from_utf8_lossy(&packet);
            info!("Message from {peer}: {message:?}");
        }

        select! {
            // Restart this loop every 100ms
            _ = (&mut timeout).fuse() => {
                timeout.reset(Duration::from_millis(100));
            }

            // Or break if the message loop ends (disconnected, closed, etc.)
            _ = &mut loop_fut1 => {
                break;
            }
        }
    }

    let mut client_a = WebRTCClientInterface::open_reliable(
        &format!("ws://127.0.0.1:{}/test", PORT),
        None,
    )
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });

    info!("client_a created");
    let client_b = WebRTCClientInterface::open_reliable(
        &format!("ws://127.0.0.1:{}/test", PORT),
        None,
    )
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to create WebRTCClientInterface: {:?}", e);
    });
    info!("client_b created");

    tokio::time::sleep(tokio::time::Duration::from_secs(7)).await;
    return;

    // FIXME lock active here
    info!("get_socket_uuid_at 1");
    assert_eq!(client_a.get_sockets_count(), 1);
    assert_eq!(client_b.get_sockets_count(), 1);
    info!("get_socket_uuid_at 2");

    let uuid = client_a.get_socket_uuid_at(0).unwrap();

    client_a.send_block(CLIENT_A_TO_CLIENT_B_MSG, uuid).await;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
}
