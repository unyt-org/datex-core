use datex_core::task::spawn;
use log::info;
use matchbox_signaling::SignalingServer;
use std::net::SocketAddr;

use datex_core::utils::context::init_global_context;

pub fn start_server(url: &str) {
    let server =
        SignalingServer::full_mesh_builder(url.parse::<SocketAddr>().unwrap())
            .on_connection_request(|e| Ok(true))
            .on_id_assignment(|(socket, id)| info!("{socket} received {id}"))
            .on_peer_connected(|id| info!("Peer connected: {id}"))
            .on_peer_disconnected(|id| info!("Peer disconnected: {id}"))
            .cors()
            .trace()
            .build();
    spawn(async move {
        server.serve().await.unwrap_or_else(|e| {
            core::panic!("Failed to start signaling server: {e:?}");
        });
    });
}

#[ignore]
#[tokio::test]
pub async fn run() {
    init_global_context();
    const PORT: u16 = 8080;
    let url = format!("127.0.0.1:{PORT}");
    start_server(&url);
    info!("Signaling server started at {url}");
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
}
