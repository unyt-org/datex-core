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

    server
        .add_channel("test", Endpoint::from_string("@jonas").unwrap())
        .await;
    // FIXME use get socket for endpoint
    let socket_uuid = server.get_socket_uuid_at(0).unwrap();
    let mut it = 0;
    while it < 10 {
        server.send_block(b"Hello World", socket_uuid.clone()).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        it += 1;
    }
}
