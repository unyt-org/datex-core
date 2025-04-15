use std::sync::{Arc, Mutex};

use super::{
    com_interface::ComInterfaceSockets,
    com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
};

// pub trait SocketProvider {
//     fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;
// }

pub trait MultipleSocketProvider {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;

    fn get_sockets_uuids(&self) -> Vec<ComInterfaceSocketUUID> {
        self.provide_sockets()
            .lock()
            .unwrap()
            .sockets
            .values()
            .map(|s| s.lock().unwrap().uuid.clone())
            .collect()
    }
    fn get_sockets_count(&self) -> usize {
        self.provide_sockets().clone().lock().unwrap().sockets.len()
    }
    // TODO
    // fn get_socket_for_endpoint(
    //     &self,
    //     endpoint: &str,
    // ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
    //     let sockets = self.provide_sockets();
    //     let sockets = sockets.lock().unwrap();
    //     let socket = sockets
    //         .sockets
    //         .values()
    //         .find(|s| s.lock().unwrap().direct_endpoint == endpoint)
    //         .cloned();
    //     socket
    // }
    fn get_socket_uuid_at(
        &self,
        index: usize,
    ) -> Option<ComInterfaceSocketUUID> {
        let sockets = self.provide_sockets();
        let sockets = sockets.lock().unwrap();
        let socket = sockets
            .sockets
            .values()
            .nth(index)
            .map(|s| s.lock().unwrap().uuid.clone());
        socket
    }
    fn get_socket_at(
        &self,
        index: usize,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        let sockets = self.provide_sockets();
        let sockets = sockets.lock().unwrap();
        let socket = sockets.sockets.values().nth(index).cloned();
        socket
    }
}

pub trait SingleSocketProvider {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>>;

    fn get_socket(&self) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        return self
            .provide_sockets()
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
