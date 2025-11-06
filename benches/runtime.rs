use datex_core::{
    runtime::{AsyncContext, Runtime, RuntimeConfig},
    values::core_values::endpoint::Endpoint,
};
use log::info;

// simple runtime initialization
pub fn runtime_init() {
    let runtime = Runtime::new(
        RuntimeConfig::new_with_endpoint(Endpoint::new("@+bench")),
        AsyncContext::new(),
    );
    info!("Runtime version: {}", runtime.version);
}
