use std::time::Duration;
use datex_core::values::value_container::ValueContainer;
use datex_core::run_async;
use datex_core::runtime::execution_context::ExecutionContext;
use datex_core::values::core_values::endpoint::Endpoint;
use crate::network::helpers::mock_setup::get_mock_setup_with_two_runtimes;

#[tokio::test]
#[ignore]
pub async fn test_basic_remote_execution() {
    run_async! {
        let endpoint_a = Endpoint::from("@test_a");
        let endpoint_b = Endpoint::from("@test_b");
        let (runtime_a, runtime_b) = get_mock_setup_with_two_runtimes(endpoint_a.clone(), endpoint_b.clone()).await;
        
        // sleep for a short time to ensure the connection is established
        tokio::time::sleep(Duration::from_millis(1000)).await;

        runtime_a.com_hub.print_metadata();
        
        // create an execution context for @test_b
        let mut remote_execution_context = ExecutionContext::remote(endpoint_b);

        // execute script remotely on @test_b
        let result = runtime_a.execute("1 + 2", &[], &mut remote_execution_context).await;
        
        assert_eq!(result.unwrap().unwrap(), ValueContainer::from(3));
    };
}