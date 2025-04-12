use std::sync::{Arc, Mutex};

use super::{
    com_interface::ComInterfaceSockets,
    com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
};

// pub trait SocketProvider {
//     fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;
// }

pub trait MultipleSocketProvider {
    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;

    fn get_sockets_uuids(&self) -> Vec<ComInterfaceSocketUUID> {
        self.get_sockets()
            .lock()
            .unwrap()
            .sockets
            .values()
            .map(|s| s.lock().unwrap().uuid.clone())
            .collect()
    }
    fn get_sockets_count(&self) -> usize {
        self.get_sockets().clone().lock().unwrap().sockets.len()
    }
    fn get_socket_uuid_at(
        &self,
        index: usize,
    ) -> Option<ComInterfaceSocketUUID> {
        self.get_sockets()
            .lock()
            .unwrap()
            .sockets
            .values()
            .nth(index)
            .map(|s| s.lock().unwrap().uuid.clone())
    }
}

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
