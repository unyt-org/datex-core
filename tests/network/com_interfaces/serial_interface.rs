use datex_core::network::com_interfaces::{
    default_com_interfaces::serial::serial_native_interface::SerialNativeInterface,
};
use log::info;

use datex_core::utils::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    init_global_context();
    const PORT_NAME: &str = "/dev/ttyUSB0";
    const BAUD_RATE: u32 = 115200;
    let available_ports = SerialNativeInterface::get_available_ports();
    for port in available_ports.clone() {
        info!("Available port: {port}");
    }
    if !available_ports.contains(&PORT_NAME.to_string()) {
        return;
    }
    let mut interface =
        SerialNativeInterface::new_with_baud_rate(PORT_NAME, BAUD_RATE)
            .unwrap_or_else(|e| {
                core::panic!("Failed to create SerialNativeInterface: {e:?}");
            });
    let socket_uuid = interface.get_socket_uuid().unwrap();
    assert!(interface.send_block(b"Hello World", socket_uuid).await);
    interface.destroy().await;
}
