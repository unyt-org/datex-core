use datex_core::runtime::{AsyncContext, Runtime, RuntimeConfig};
use log::info;

// simple runtime initialization
pub fn runtime_init() {
    let runtime = Runtime::new(RuntimeConfig::default(), AsyncContext::new());
    info!("Runtime version: {}", runtime.version);
}
