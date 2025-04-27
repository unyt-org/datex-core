use std::sync::{Arc, Mutex};

use crate::datex_values::Endpoint;

use super::{
    com_interface::ComInterfaceSockets,
    com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
};
// TODO we can put them to the datex_core as macro
// We might consider using #[com_interface(multiple)] and #[com_interface(single)]
// to generate the code for us
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

    fn get_socket_uuid_for_endpoint(
        &self,
        endpoint: Endpoint,
    ) -> Option<ComInterfaceSocketUUID> {
        let sockets = self.provide_sockets();
        let sockets = sockets.lock().unwrap();
        let socket = sockets
            .sockets
            .values()
            .find(|s| {
                s.lock().unwrap().direct_endpoint == Some(endpoint.clone())
            })
            .map(|s| s.lock().unwrap().uuid.clone());
        socket
    }
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

    fn has_socket_with_uuid(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> bool {
        let sockets = self.provide_sockets();
        let sockets = sockets.lock().unwrap();
        sockets.sockets.contains_key(&socket_uuid)
    }

    fn get_socket_with_uuid(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        let sockets = self.provide_sockets();
        let sockets = sockets.lock().unwrap();
        let socket = sockets.sockets.get(&socket_uuid).cloned();
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
    fn has_socket_with_uuid(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> bool {
        self.get_socket_uuid()
            .map(|uuid| uuid == socket_uuid)
            .unwrap_or(false)
    }
}
