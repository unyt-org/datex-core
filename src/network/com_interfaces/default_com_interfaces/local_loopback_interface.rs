use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::datex_values::Endpoint;
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

use super::super::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::ComInterfaceState;

/// A simple local loopback interface that puts outgoing data
/// back into the incoming queue.
pub struct LocalLoopbackInterface {
    socket: Arc<Mutex<ComInterfaceSocket>>,
    info: ComInterfaceInfo,
}
impl LocalLoopbackInterface {
    pub fn new() -> LocalLoopbackInterface {
        let mut info = ComInterfaceInfo::new();
        info.set_state(ComInterfaceState::Connected);

        let socket = Arc::new(Mutex::new(ComInterfaceSocket::new(
            info.get_uuid().clone(),
            InterfaceDirection::IN_OUT,
            1,
        )));

        let uuid = socket.lock().unwrap().uuid.clone();
        info.com_interface_sockets().lock().unwrap()
            .add_socket(socket.clone());
        info.com_interface_sockets().lock().unwrap()
            .register_socket_endpoint(
                uuid,
                Endpoint::LOCAL,
                1
            ).expect("Could not register endpoint to local socket");
        
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
        let socket = socket.lock().unwrap();
        socket.get_receive_queue().lock().unwrap().extend(block);
        Box::pin(async { true })
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "local".to_string(),
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            ..InterfaceProperties::default()
        }
    }
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }
    delegate_com_interface_info!();
}
