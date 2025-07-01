use log::info;
use datex_core::runtime::Runtime;

// simple runtime initialization
pub fn runtime_init() {
    let runtime = Runtime::default();
    info!("Runtime version: {}", runtime.version);
}
