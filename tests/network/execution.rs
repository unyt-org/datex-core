use datex_core::datex_values::value_container::ValueContainer;
use datex_core::run_async;
use datex_core::runtime::execution_context::ExecutionContext;
use datex_core::runtime::Runtime;

#[tokio::test]
#[ignore]
pub async fn test_basic_remote_execution() {
    run_async! {
        let runtime_a = Runtime::init_native("@test_a");
        let runtime_b = Runtime::init_native("@test_b");

        // TODO: connect the runtimes

        // create an execution context for @test_b
        let remote_execution_context = ExecutionContext::remote("@test_b");

        // execute script remotely on @test_b
        let result = runtime_a.execute("1 + 2", &[], remote_execution_context).await;

        assert_eq!(result.unwrap().unwrap(), ValueContainer::from(3));
    };
}