use datex_core::{runtime::Runtime, utils::{logger::LoggerContext, buffers::{buffer_to_hex, hex_to_buffer, hex_to_buffer_advanced}}};


#[test]
pub fn init_runtime() {
    let runtime = Runtime::new();
    assert_eq!(runtime.version, 1);
}


#[test]
pub fn execute_block() {
    // let runtime = Runtime::new();
    // let dxb = hex_to_buffer_advanced("01 64 02 00 00 ff 01 02".to_string(), " ");
    // runtime.execute(&dxb)
}