use std::time::Duration;
use log::info;
use ntest_timeout::timeout;
use tokio::task;
use datex_core::network::com_interfaces::com_interface::ComInterfaceFactory;
use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{TEST_ENDPOINT_A, TEST_ENDPOINT_B, TEST_ENDPOINT_C, TEST_ENDPOINT_D};
use crate::network::helpers::mockup_interface::{MockupInterface, MockupInterfaceSetupData};
use crate::network::helpers::network::{InterfaceConnection, Network, Node};


#[tokio::test]
#[timeout(100)]
async fn create_network_with_two_nodes() {
    let local = task::LocalSet::new();
    local.run_until(async {
        init_global_context();

        let mut network = Network::create(
            vec![
                // @test-a
                Node::new(TEST_ENDPOINT_A.clone())
                    .with_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("ab")
                    )),
                // @test-b
                Node::new(TEST_ENDPOINT_B.clone())
                    .with_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("ab")
                    ))
            ]
        );
        network.register_interface(
            "mockup",
            MockupInterface::factory
        );

        network.start().await;

        // sleep 100ms
        tokio::time::sleep(Duration::from_millis(10)).await;

        info!("Network started");

        for endpoint in network.endpoints.iter() {
            if let Some(runtime) = &endpoint.runtime {
                runtime.com_hub.print_metadata();
            }
        }

        let runtime_a = network.get_runtime(TEST_ENDPOINT_A.clone());
        let runtime_b = network.get_runtime(TEST_ENDPOINT_B.clone());

        // send trace from A to B
        let network_trace = runtime_a.com_hub.record_trace(
            TEST_ENDPOINT_B.clone()
        ).await;
        assert!(network_trace.is_some());
        info!("Network trace:\n{}", network_trace.as_ref().unwrap());
        assert!(network_trace.unwrap().matches_hops(&[
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup")
        ]));

        // send trace from B to A
        let network_trace = runtime_b.com_hub.record_trace(
            TEST_ENDPOINT_A.clone()
        ).await;
        assert!(network_trace.is_some());
        info!("Network trace:\n{}", network_trace.as_ref().unwrap());
        assert!(network_trace.unwrap().matches_hops(&[
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup")
        ]));

        // send trace from A to A
        let network_trace = runtime_a.com_hub.record_trace(
            TEST_ENDPOINT_A.clone()
        ).await;
        assert!(network_trace.is_some());
        info!("Network trace:\n{}", network_trace.as_ref().unwrap());
        assert!(network_trace.unwrap().matches_hops(&[
            (TEST_ENDPOINT_A.clone(), "local"),
            (TEST_ENDPOINT_A.clone(), "local"),
            (TEST_ENDPOINT_A.clone(), "local"),
            (TEST_ENDPOINT_A.clone(), "local")
        ]));

    }).await;
}


#[tokio::test]
#[timeout(100)]
async fn create_network_with_three_nodes() {
    let local = task::LocalSet::new();
    local.run_until(async {
        init_global_context();

        let mut network = Network::create(
            vec![
                // @test-a
                Node::new(TEST_ENDPOINT_A.clone())
                    .with_default_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("ab")
                    )),
                // @test-b
                Node::new(TEST_ENDPOINT_B.clone())
                    .with_default_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("ab")
                    ))
                    .with_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("bc")
                    )),
                // @test-c
                Node::new(TEST_ENDPOINT_C.clone())
                    .with_default_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("bc")
                    ))
                    .with_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("cd")
                    )),
                // @test-d
                Node::new(TEST_ENDPOINT_D.clone())
                    .with_default_connection(InterfaceConnection::new(
                        "mockup",
                        MockupInterfaceSetupData::new("cd")
                    ))
            ]
        );
        network.register_interface(
            "mockup",
            MockupInterface::factory
        );

        network.start().await;

        // sleep 100ms
        tokio::time::sleep(Duration::from_millis(10)).await;

        info!("Network started");

        for endpoint in network.endpoints.iter() {
            if let Some(runtime) = &endpoint.runtime {
                runtime.com_hub.print_metadata();
            }
        }

        let runtime_a = network.get_runtime(TEST_ENDPOINT_A.clone());
        let runtime_b = network.get_runtime(TEST_ENDPOINT_B.clone());
        let runtime_c = network.get_runtime(TEST_ENDPOINT_C.clone());

        // send trace from A to C
        let network_trace = runtime_a.com_hub.record_trace(
            TEST_ENDPOINT_C.clone(),
        ).await;
        assert!(network_trace.is_some());
        info!("Network trace:\n{}", network_trace.as_ref().unwrap());
        assert!(network_trace.unwrap().matches_hops(&[
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_C.clone(), "mockup"),
            (TEST_ENDPOINT_C.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup")
        ]));

        // send trace from C to A
        let network_trace = runtime_c.com_hub.record_trace(
            TEST_ENDPOINT_A.clone(),
        ).await;
        assert!(network_trace.is_some());
        info!("Network trace:\n{}", network_trace.as_ref().unwrap());
        assert!(network_trace.unwrap().matches_hops(&[
            (TEST_ENDPOINT_C.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_C.clone(), "mockup")
        ]));
        
        // send trace from A to D
        let network_trace = runtime_a.com_hub.record_trace(
            TEST_ENDPOINT_D.clone(),
        ).await;
        assert!(network_trace.is_some());
        info!("Network trace:\n{}", network_trace.as_ref().unwrap());
        assert!(network_trace.unwrap().matches_hops(&[
            (TEST_ENDPOINT_A.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_C.clone(), "mockup"),
            (TEST_ENDPOINT_C.clone(), "mockup"),
            (TEST_ENDPOINT_D.clone(), "mockup"),
            (TEST_ENDPOINT_D.clone(), "mockup"),
            (TEST_ENDPOINT_C.clone(), "mockup"),
            (TEST_ENDPOINT_C.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_B.clone(), "mockup"),
            (TEST_ENDPOINT_A.clone(), "mockup")
        ]));

    }).await;
}