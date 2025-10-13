use datex_core::runtime::Runtime;
use log::info;

// simple runtime initialization
pub fn runtime_init() {
    let runtime = Runtime::default();
    info!("Runtime version: {}", runtime.version);
}
