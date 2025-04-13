use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::delegate_socket_state;
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
    com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
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

        LocalLoopbackInterface {
            com_interface_sockets: Arc::new(Mutex::new(sockets)),
            info,
            socket,
        }
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "local".to_string(),
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            ..InterfaceProperties::default()
        }
    }

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
    delegate_socket_state!();
}
