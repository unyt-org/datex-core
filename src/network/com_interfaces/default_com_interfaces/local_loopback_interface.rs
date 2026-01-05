use super::super::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::ComInterfaceInfo;
use crate::network::com_interfaces::com_interface::ComInterfaceState;
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::std_sync::Mutex;
use crate::stdlib::boxed::Box;
use crate::stdlib::pin::Pin;
use crate::stdlib::string::ToString;
use crate::stdlib::sync::Arc;
use crate::values::core_values::endpoint::Endpoint;
use crate::{delegate_com_interface_info, set_sync_opener};
use core::future::Future;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::time::Duration;
use datex_macros::{com_interface, create_opener};

/// A simple local loopback interface that puts outgoing data
/// back into the incoming queue.
pub struct LocalLoopbackInterface {
    socket: Arc<Mutex<ComInterfaceSocket>>,
    info: ComInterfaceInfo,
}
impl Default for LocalLoopbackInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[com_interface]
impl LocalLoopbackInterface {
    pub fn new() -> LocalLoopbackInterface {
        let info = ComInterfaceInfo::new();
        let socket = Arc::new(Mutex::new(ComInterfaceSocket::init(
            info.get_uuid().clone(),
            InterfaceDirection::InOut,
            1,
        )));
        LocalLoopbackInterface { info, socket }
    }

    #[create_opener]
    fn open(&mut self) -> Result<(), ()> {
        let uuid = self.socket.try_lock().unwrap().uuid.clone();
        self.add_socket(self.socket.clone());
        self.register_socket_endpoint(uuid, Endpoint::LOCAL, 1)
            .unwrap();
        Ok(())
    }
}

impl ComInterface for LocalLoopbackInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        log::info!("LocalLoopbackInterface Sending block: {block:?}");
        let socket = self.socket.try_lock().unwrap();
        log::info!("LocalLoopbackInterface sent block");
        socket.get_receive_queue().try_lock().unwrap().extend(block);
        Box::pin(async { true })
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "local".to_string(),
            channel: "local".to_string(),
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            ..InterfaceProperties::default()
        }
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }
    delegate_com_interface_info!();
    set_sync_opener!(open);
}
