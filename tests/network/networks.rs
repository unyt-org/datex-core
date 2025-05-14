use crate::context::init_global_context;
use crate::network::helpers::mock_setup::{
    TEST_ENDPOINT_A, TEST_ENDPOINT_B, TEST_ENDPOINT_C, TEST_ENDPOINT_D,
};
use crate::network::helpers::mockup_interface::{
    MockupInterface, MockupInterfaceSetupData,
};
use crate::network::helpers::network::{
    InterfaceConnection, Network, Node, Route,
};
use datex_core::datex_values::Endpoint;
use datex_core::network::com_hub::InterfacePriority;
use datex_core::network::com_interfaces::com_interface::ComInterfaceFactory;
use datex_core::run_async;
use log::info;
use ntest_timeout::timeout;
use std::str::FromStr;
use std::time::Duration;
use tokio::task;

#[tokio::test]
#[timeout(100)]
async fn create_network_with_two_nodes() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();

            let mut network = Network::create(vec![
                // @test-a
                Node::new(TEST_ENDPOINT_A.clone()).with_connection(
                    InterfaceConnection::new(
                        "mockup",
                        InterfacePriority::default(),
                        MockupInterfaceSetupData::new("ab"),
                    ),
                ),
                // @test-b
                Node::new(TEST_ENDPOINT_B.clone()).with_connection(
                    InterfaceConnection::new(
                        "mockup",
                        InterfacePriority::default(),
                        MockupInterfaceSetupData::new("ab"),
                    ),
                ),
            ]);
            network.register_interface("mockup", MockupInterface::factory);

            network.start().await;

            // sleep 100ms
            tokio::time::sleep(Duration::from_millis(10)).await;

            info!("Network started");

            let runtime_a = network.get_runtime(TEST_ENDPOINT_A.clone());
            let runtime_b = network.get_runtime(TEST_ENDPOINT_B.clone());

            // send trace from A to B
            let network_trace = runtime_a
                .com_hub
                .record_trace(TEST_ENDPOINT_B.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());
            assert!(network_trace.unwrap().matches_hops(&[
                (TEST_ENDPOINT_A.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_A.clone(), "mockup")
            ]));

            // send trace from B to A
            let network_trace = runtime_b
                .com_hub
                .record_trace(TEST_ENDPOINT_A.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());
            assert!(network_trace.unwrap().matches_hops(&[
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_A.clone(), "mockup"),
                (TEST_ENDPOINT_A.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup")
            ]));

            // send trace from A to A
            let network_trace = runtime_a
                .com_hub
                .record_trace(TEST_ENDPOINT_A.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());
            assert!(network_trace.unwrap().matches_hops(&[
                (TEST_ENDPOINT_A.clone(), "local"),
                (TEST_ENDPOINT_A.clone(), "local"),
                (TEST_ENDPOINT_A.clone(), "local"),
                (TEST_ENDPOINT_A.clone(), "local")
            ]));
        })
        .await;
}

async fn get_test_network_1() -> Network {
    let mut network = Network::create(vec![
        // @test-a
        Node::new(TEST_ENDPOINT_A.clone()).with_connection(
            InterfaceConnection::new(
                "mockup",
                InterfacePriority::default(),
                MockupInterfaceSetupData::new("ab"),
            ),
        ),
        // @test-b
        Node::new(TEST_ENDPOINT_B.clone())
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::default(),
                MockupInterfaceSetupData::new("ab"),
            ))
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::default(),
                MockupInterfaceSetupData::new("bc"),
            )),
        // @test-c
        Node::new(TEST_ENDPOINT_C.clone())
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::default(),
                MockupInterfaceSetupData::new("bc"),
            ))
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::default(),
                MockupInterfaceSetupData::new("cd"),
            )),
        // @test-d
        Node::new(TEST_ENDPOINT_D.clone()).with_connection(
            InterfaceConnection::new(
                "mockup",
                InterfacePriority::default(),
                MockupInterfaceSetupData::new("cd"),
            ),
        ),
    ]);
    network.register_interface("mockup", MockupInterface::factory);

    network.start().await;
    network
}

async fn get_test_network_1_with_deterministic_priorities() -> Network {
    let mut network = Network::create(vec![
        // @test-a
        Node::new(TEST_ENDPOINT_A.clone()).with_connection(
            InterfaceConnection::new(
                "mockup",
                InterfacePriority::Priority(0),
                MockupInterfaceSetupData::new("ab"),
            ),
        ),
        // @test-b
        Node::new(TEST_ENDPOINT_B.clone())
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::Priority(0),
                MockupInterfaceSetupData::new("ab"),
            ))
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::Priority(1),
                MockupInterfaceSetupData::new("bc"),
            )),
        // @test-c
        Node::new(TEST_ENDPOINT_C.clone())
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::Priority(0),
                MockupInterfaceSetupData::new("bc"),
            ))
            .with_connection(InterfaceConnection::new(
                "mockup",
                InterfacePriority::Priority(1),
                MockupInterfaceSetupData::new("cd"),
            )),
        // @test-d
        Node::new(TEST_ENDPOINT_D.clone()).with_connection(
            InterfaceConnection::new(
                "mockup",
                InterfacePriority::Priority(0),
                MockupInterfaceSetupData::new("cd"),
            ),
        ),
    ]);
    network.register_interface("mockup", MockupInterface::factory);

    network.start().await;
    network
}

#[cfg(feature = "debug")]
#[tokio::test]
#[timeout(1000)]
async fn network_routing_with_four_nodes_1() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();

            let network = get_test_network_1().await;

            // sleep 100ms
            tokio::time::sleep(Duration::from_millis(20)).await;

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
            let network_trace = runtime_a
                .com_hub
                .record_trace(TEST_ENDPOINT_C.clone())
                .await;
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
        })
        .await;
}

#[cfg(feature = "debug")]
#[tokio::test]
#[timeout(1000)]
async fn network_routing_with_four_nodes_2() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();

            let network = get_test_network_1().await;

            // sleep 100ms
            tokio::time::sleep(Duration::from_millis(20)).await;

            info!("Network started");

            for endpoint in network.endpoints.iter() {
                if let Some(runtime) = &endpoint.runtime {
                    runtime.com_hub.print_metadata();
                }
            }

            let runtime_a = network.get_runtime(TEST_ENDPOINT_A.clone());
            let runtime_b = network.get_runtime(TEST_ENDPOINT_B.clone());
            let runtime_c = network.get_runtime(TEST_ENDPOINT_C.clone());

            // send trace from C to A
            // this first trace does not route deterministically depending on the
            // order in the priority list
            // after the first trace, the routing should be deterministic
            let network_trace = runtime_c
                .com_hub
                .record_trace(TEST_ENDPOINT_A.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());

            // clear endpoint blacklist to make sure it has no influence on the following routing
            runtime_c
                .com_hub
                .endpoint_sockets_blacklist
                .borrow_mut()
                .clear();

            // send trace from C to A again
            let network_trace = runtime_c
                .com_hub
                .record_trace(TEST_ENDPOINT_A.clone())
                .await;
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
        })
        .await;
}

#[cfg(feature = "debug")]
#[tokio::test]
#[timeout(1000)]
async fn network_routing_with_four_nodes_3() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();

            let network = get_test_network_1().await;

            // sleep 100ms
            tokio::time::sleep(Duration::from_millis(20)).await;

            info!("Network started");

            for endpoint in network.endpoints.iter() {
                if let Some(runtime) = &endpoint.runtime {
                    runtime.com_hub.print_metadata();
                }
            }

            let runtime_a = network.get_runtime(TEST_ENDPOINT_A.clone());
            let runtime_b = network.get_runtime(TEST_ENDPOINT_B.clone());
            let runtime_c = network.get_runtime(TEST_ENDPOINT_C.clone());

            // send trace from A to D
            let network_trace = runtime_a
                .com_hub
                .record_trace(TEST_ENDPOINT_D.clone())
                .await;
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
        })
        .await;
}

#[cfg(feature = "debug")]
#[tokio::test]
#[timeout(1000)]
async fn network_routing_with_four_nodes_4() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();

            let network = get_test_network_1().await;

            // sleep 100ms
            tokio::time::sleep(Duration::from_millis(20)).await;

            info!("Network started");

            for endpoint in network.endpoints.iter() {
                if let Some(runtime) = &endpoint.runtime {
                    runtime.com_hub.print_metadata();
                }
            }

            let runtime_a = network.get_runtime(TEST_ENDPOINT_A.clone());
            let runtime_b = network.get_runtime(TEST_ENDPOINT_B.clone());
            let runtime_c = network.get_runtime(TEST_ENDPOINT_C.clone());

            // send trace from B to D
            // this first trace does not route deterministically depending on the
            // order in the priority list
            // after the first trace, the routing should be deterministic
            let network_trace = runtime_b
                .com_hub
                .record_trace(TEST_ENDPOINT_D.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());

            // clear endpoint blacklist to make sure it has no influence on the following routing
            runtime_c
                .com_hub
                .endpoint_sockets_blacklist
                .borrow_mut()
                .clear();

            // send trace from B to D again
            let network_trace = runtime_b
                .com_hub
                .record_trace(TEST_ENDPOINT_D.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());
            assert!(network_trace.unwrap().matches_hops(&[
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_D.clone(), "mockup"),
                (TEST_ENDPOINT_D.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup")
            ]));
        })
        .await;
}

#[cfg(feature = "debug")]
#[tokio::test]
#[timeout(1000)]
async fn network_routing_with_four_nodes_5_deterministic_priorities() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();

            let network =
                get_test_network_1_with_deterministic_priorities().await;

            // sleep 100ms
            tokio::time::sleep(Duration::from_millis(20)).await;

            info!("Network started");

            for endpoint in network.endpoints.iter() {
                if let Some(runtime) = &endpoint.runtime {
                    runtime.com_hub.print_metadata();
                }
            }

            let runtime_a = network.get_runtime(TEST_ENDPOINT_A.clone());
            let runtime_b = network.get_runtime(TEST_ENDPOINT_B.clone());
            let runtime_c = network.get_runtime(TEST_ENDPOINT_C.clone());

            // send trace from B to D

            let network_trace = runtime_b
                .com_hub
                .record_trace(TEST_ENDPOINT_D.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());
            assert!(network_trace.unwrap().matches_hops(&[
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_D.clone(), "mockup"),
                (TEST_ENDPOINT_D.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup")
            ]));
        })
        .await;
}

#[cfg(feature = "debug")]
#[tokio::test]
#[timeout(1000)]
async fn network_routing_with_four_nodes_6_deterministic_priorities() {
    let local = task::LocalSet::new();
    local
        .run_until(async {
            init_global_context();

            let network =
                get_test_network_1_with_deterministic_priorities().await;

            // sleep 100ms
            tokio::time::sleep(Duration::from_millis(20)).await;

            info!("Network started");

            for endpoint in network.endpoints.iter() {
                if let Some(runtime) = &endpoint.runtime {
                    runtime.com_hub.print_metadata();
                }
            }

            let runtime_c = network.get_runtime(TEST_ENDPOINT_C.clone());

            // send trace from C A

            let network_trace = runtime_c
                .com_hub
                .record_trace(TEST_ENDPOINT_A.clone())
                .await;
            assert!(network_trace.is_some());
            info!("Network trace:\n{}", network_trace.as_ref().unwrap());
            assert!(network_trace.unwrap().matches_hops(&[
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_D.clone(), "mockup"),
                (TEST_ENDPOINT_D.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_A.clone(), "mockup"),
                (TEST_ENDPOINT_A.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_B.clone(), "mockup"),
                (TEST_ENDPOINT_C.clone(), "mockup"),
            ]));
        })
        .await;
}

#[tokio::test]
#[timeout(3000)]
async fn simple_network() {
    init_global_context();
    run_async! {
        let mut network = Network::load(
            "simple.json",
        );
        network.start().await;
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Route::between("@4726", "@s5zw")
            .to_via("@yhr9", "mockup")
            .hop("@s5zw")
            .hop("@4726")
            .test(&network)
            .await
    };
}

#[tokio::test]
#[timeout(7000)]
async fn complex_network_1() {
    init_global_context();
    run_async! {
        let mut network = Network::load(
            "complex.json",
        );
        network.start().await;
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Route::between("@bk2y", "@n7oe")
            .hop("@em68")
            .hop("@msun")
            .hop("@fyig")
            .hop("@n7oe")
            .hop("@fyig")
            .hop("@msun")
            .hop("@ajil")
            .hop("@bk2y")
            .test(&network)
            .await
    }
}

#[tokio::test]
#[timeout(7000)]
async fn complex_network_2() {
    init_global_context();
    run_async! {
        let mut network = Network::load(
            "complex.json",
        );
        network.start().await;
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Route::between("@msun", "@bk2y")
            .hop("@fyig")
            .hop("@n7oe")
            .hop("@fyig")
            .hop("@msun")
            .hop("@ajil")
            .hop("@bk2y")
            .hop("@em68")
            .hop("@msun")
            .test(&network)
            .await
    }
}

#[tokio::test]
#[timeout(7000)]
async fn complex_network_3() {
    init_global_context();
    run_async! {
        let mut network = Network::load(
            "complex.json",
        );
        network.start().await;
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Route::between("@fyig", "@n7oe")
            .hop("@n7oe")
            .hop("@fyig")
            .test(&network)
            .await
    }
}

#[tokio::test]
#[timeout(7000)]
async fn threesome_1() {
    init_global_context();
    run_async! {
        let mut network = Network::load(
            "threesome.json",
        );
        network.start().await;
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Route::between("@msun", "@n7oe")
            .hop("@em68")
            .hop("@msun")
            .hop("@ajil")
            .hop("@msun")
            .hop("@fyig")
            .hop("@n7oe")
            .hop("@fyig")
            .hop("@msun")
            .test(&network)
            .await
    }
}
