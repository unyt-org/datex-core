use std::{thread};
use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{get_mock_setup_and_socket_for_endpoint_and_update_loop, TEST_ENDPOINT_A, TEST_ENDPOINT_B};
use std::sync::{mpsc};
use ntest_timeout::timeout;
use tokio::task;
use datex_core::network::com_hub::InterfacePriority;

#[tokio::test(flavor = "current_thread")]
#[timeout(1000)]
async fn create_network_trace() {
    let local = task::LocalSet::new();
    local.run_until(async {
        init_global_context();

        let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
        let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

        let (com_hub_mut_a, com_interface_a, socket_a) =
            get_mock_setup_and_socket_for_endpoint_and_update_loop(
                TEST_ENDPOINT_A.clone(),
                None,
                Some(sender_a),
                Some(receiver_b),
                InterfacePriority::default(),
                true
            )
                .await;

        let (com_hub_mut_b, com_interface_b, socket_b) =
            get_mock_setup_and_socket_for_endpoint_and_update_loop(
                TEST_ENDPOINT_B.clone(),
                None,
                Some(sender_b),
                Some(receiver_a),
                InterfacePriority::default(),
                true
            )
                .await;

        com_hub_mut_a.update().await;
        com_hub_mut_b.update().await;
        com_interface_a.borrow_mut().update();
        com_interface_b.borrow_mut().update();
        com_hub_mut_a.update().await;
        com_hub_mut_b.update().await;

        log::info!("Sending trace from A to B");

        // send trace from A to B
        let network_trace = com_hub_mut_a.record_trace(
            TEST_ENDPOINT_B.clone(),
        ).await;

        assert!(network_trace.is_some());
        log::info!("Network trace:\n{}", network_trace.as_ref().unwrap());

        assert!(network_trace.unwrap().matches_hops(&[
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup")
        ]));

    }).await;
}

// same as create_network_trace, but both com hubs in separate threads
#[tokio::test]
#[timeout(3000)]
async fn create_network_trace_separate_threads() {
    // create a new thread for each com hub
    let (sender_a, receiver_a) = mpsc::channel::<Vec<u8>>();
    let (sender_b, receiver_b) = mpsc::channel::<Vec<u8>>();

    // Endpoint A
    let thread_a = thread::spawn(move || {
        // tokio runtime setup
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Run an async block using the runtime
        runtime.block_on(async {
            let local = task::LocalSet::new();

            local.run_until(async {
                init_global_context();

                let (com_hub_mut_a, com_interface_a, socket_a) =
                    get_mock_setup_and_socket_for_endpoint_and_update_loop(
                        TEST_ENDPOINT_A.clone(),
                        None,
                        Some(sender_a),
                        Some(receiver_b),
                        InterfacePriority::default(),
                        true
                    ).await;


                log::info!("Sending trace from A to B");
                // sleep required to handle message transfer
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // send trace from A to B
                let network_trace = com_hub_mut_a.record_trace(
                    TEST_ENDPOINT_B.clone(),
                ).await;

                assert!(network_trace.is_some());
                log::info!("Network trace:\n{}", network_trace.as_ref().unwrap());

                assert!(network_trace.unwrap().matches_hops(&[
                    (TEST_ENDPOINT_A.clone(), "mockup"),
                    (TEST_ENDPOINT_B.clone(), "mockup"),
                    (TEST_ENDPOINT_B.clone(), "mockup"),
                    (TEST_ENDPOINT_A.clone(), "mockup")
                ]));
            }).await;
        });
    });

    // Endpoint B
    let thread_b = thread::spawn(move || {
        // tokio runtime setup
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Run an async block using the runtime
        runtime.block_on(async {
            let local = task::LocalSet::new();

            local.run_until(async {
                init_global_context();

                let (com_hub_mut_b, com_interface_b, socket_b) =
                    get_mock_setup_and_socket_for_endpoint_and_update_loop(
                        TEST_ENDPOINT_B.clone(),
                        None,
                        Some(sender_b),
                        Some(receiver_a),
                        InterfacePriority::default(),
                        true
                    ).await;

                // sleep 2s to ensure that the other thread has finished
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }).await;
        });
    });

    // Wait for both threads to finish
    thread_a.join().expect("Thread A panicked");
    thread_b.join().expect("Thread B panicked");
}