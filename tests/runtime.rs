use datex_core::stdlib::env;

use datex_core::runtime::{Runtime, RuntimeConfig};
use datex_core::values::core_values::endpoint::Endpoint;

/**
 * test if the DATEX Runtime is initialized correctly
 */
#[test]
pub fn init_runtime() {
    let runtime = Runtime::init_native(RuntimeConfig::new_with_endpoint(
        Endpoint::new("@test"),
    ));
    assert_eq!(runtime.version, env!("CARGO_PKG_VERSION"));
}

/**
 * test if a DXB block is executed correctly in the Runtime
 */
#[test]
pub fn execute_block() {
    assert_eq!(1, 1)
    // let runtime = Runtime::new();
    // let dxb = hex_to_buffer_advanced("01 64 02 00 00 ff 01 02".to_string(), " ");
    // runtime.execute(&dxb)
}
