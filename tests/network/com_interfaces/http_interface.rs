use core::str::FromStr;
use datex_core::{
    network::com_interfaces::{
        default_com_interfaces::http::http_server_interface::HTTPServerNativeInterface,
    },
    values::core_values::endpoint::Endpoint,
};
use datex_core::network::com_interfaces::com_interface::ComInterface;
use datex_core::network::com_interfaces::default_com_interfaces::http::http_common::HTTPServerInterfaceSetupData;
use datex_core::utils::context::init_global_context;

// $ head -c 48192 /dev/zero | curl -X POST http://localhost:8081/my-secret-channel/tx --data-binary @-
#[tokio::test]
pub async fn test_construct() {
    const PORT: u16 = 8081;
    init_global_context();

    let server = ComInterface::create_with_implementation::<HTTPServerNativeInterface>(
        HTTPServerInterfaceSetupData {port: PORT}
    ).expect("Failed to create HTTP server interface");
    assert!(server.borrow().handle_open().await);

    let endpoint = Endpoint::from_str("@jonas").unwrap();

    // TODO: add as_any downcast?

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
