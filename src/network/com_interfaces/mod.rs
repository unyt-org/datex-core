mod block_collector;
pub mod com_interface;
pub mod com_interface_properties;
pub mod com_interface_socket;
pub mod websocket_client;

#[cfg(not(any(target_arch = "wasm32", target_arch = "xtensa")))]
pub mod default_com_interfaces;
