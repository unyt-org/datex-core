use datex_core::{
    datex_values::Endpoint,
    network::com_interfaces::{
        com_interface::ComInterface,
        default_com_interfaces::http::http_server_interface::HTTPServerNativeInterface,
        socket_provider::MultipleSocketProvider,
    },
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8081;
    init_global_context();

    let mut server = HTTPServerNativeInterface::open(&PORT)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to create HTTPServerInterface: {:?}", e);
        });

    let endpoint = Endpoint::from_string("@jonas").unwrap();

    server
        .add_channel("my-secret-channel", endpoint.clone())
        .await;
    let socket_uuid = server.get_socket_uuid_for_endpoint(endpoint).unwrap();
    let mut it = 0;

    // head -c 48192 /dev/zero | curl -X POST http://localhost:8081/test/tx --data-binary @-

    while it < 10 {
        server.send_block(b"Hello World", socket_uuid.clone()).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        it += 1;
    }

    server.remove_channel("my-secret-channel").await;
    server.close().await;
}
