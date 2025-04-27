use std::sync::mpsc;
use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{get_mock_setup_with_socket_and_endpoint, TEST_ENDPOINT_A, TEST_ENDPOINT_B};

#[tokio::test]
async fn create_network_trace() {
    init_global_context();

    init_global_context();
    let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
    let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

    let (com_hub_mut_a, com_interface_a, socket_a) =
        get_mock_setup_with_socket_and_endpoint(
            TEST_ENDPOINT_A.clone(),
            None,
            Some(sender_a),
            Some(receiver_b),
        )
            .await;

    let (com_hub_mut_b, com_interface_b, socket_b) =
        get_mock_setup_with_socket_and_endpoint(
            TEST_ENDPOINT_B.clone(),
            None,
            Some(sender_b),
            Some(receiver_a),
        )
            .await;


    let network_trace = com_hub_mut_a
        .lock()
        .unwrap()
        .create_network_trace(TEST_ENDPOINT_B.clone());

    com_hub_mut_a.lock().unwrap().update().await;
    com_interface_b.borrow_mut().update();
    com_hub_mut_b.lock().unwrap().update().await;
    
    assert!(network_trace.is_some());
}