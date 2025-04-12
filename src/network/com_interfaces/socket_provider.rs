use std::sync::{Arc, Mutex};

use super::{
    com_interface::ComInterfaceSockets,
    com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
};

pub trait SingleSocketProvider {
    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;

    fn get_socket(&self) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        return self
            .get_sockets()
            .lock()
            .unwrap()
            .sockets
            .values()
            .next()
            .cloned();
    }

    fn get_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.get_socket().map(|s| s.lock().unwrap().uuid.clone())
    }
}
