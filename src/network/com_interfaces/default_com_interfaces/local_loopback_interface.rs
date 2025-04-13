use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::delegate_com_interface_info;
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::utils::uuid::UUID;

use super::super::com_interface::ComInterface;

/// A simple local loopback interface that puts outgoing data
/// back into the incoming queue.
pub struct LocalLoopbackInterface {
    socket: Arc<Mutex<ComInterfaceSocket>>,
    info: ComInterfaceInfo,
}
impl LocalLoopbackInterface {
    pub async fn new() -> LocalLoopbackInterface {
        let info = ComInterfaceInfo::new();

        let mut sockets = ComInterfaceSockets::default();
        let socket = Arc::new(Mutex::new(ComInterfaceSocket::new(
            info.get_uuid().clone(),
            InterfaceDirection::IN_OUT,
            1,
        )));
        sockets.add_socket(socket.clone());

        LocalLoopbackInterface { info, socket }
    }
}

impl ComInterface for LocalLoopbackInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let socket = self.socket.clone();
        let mut socket = socket.lock().unwrap();
        socket.get_receive_queue().lock().unwrap().extend(block);
        Box::pin(async { true })
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "local".to_string(),
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            ..InterfaceProperties::default()
        }
    }

    delegate_com_interface_info!();
}
