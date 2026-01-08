use core::str::FromStr;
use datex_core::{
    network::com_interfaces::{
        com_interface_old::ComInterfaceOld,
        default_com_interfaces::http::http_server_interface::HTTPServerNativeInterface,
        socket_provider::MultipleSocketProvider,
    },
    values::core_values::endpoint::Endpoint,
};

use crate::context::init_global_context;

// $ head -c 48192 /dev/zero | curl -X POST http://localhost:8081/my-secret-channel/tx --data-binary @-
#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8081;
    init_global_context();

    let mut server =
        HTTPServerNativeInterface::new(&PORT).unwrap_or_else(|e| {
            core::panic!("Failed to create HTTPServerInterface: {e:?}");
        });
    server.open().await.unwrap();
    let endpoint = Endpoint::from_str("@jonas").unwrap();

    server
        .add_channel("my-secret-channel", endpoint.clone())
        .await;
    let socket_uuid = server.get_socket_uuid_for_endpoint(endpoint).unwrap();
    let mut it = 0;

    while it < 5 {
        server.send_block(b"Hello World", socket_uuid.clone()).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        it += 1;
    }

    server.remove_channel("my-secret-channel").await;
    server.destroy().await;
}
