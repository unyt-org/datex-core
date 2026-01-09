use crate::network::com_interfaces::com_interface::socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::com_interface::socket_manager::ComInterfaceSocketManager;
use crate::std_sync::Mutex;
use crate::stdlib::sync::Arc;
use crate::stdlib::vec::Vec;
use crate::values::core_values::endpoint::Endpoint;
use core::prelude::rust_2024::*;

// TODO #197 we can put them to the datex_core as macro
// We might consider using #[com_interface(multiple)] and #[com_interface(single)]
// to generate the code for us
pub trait MultipleSocketProvider {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSocketManager>>;

    fn get_sockets_uuids(&self) -> Vec<ComInterfaceSocketUUID> {
        self.provide_sockets()
            .try_lock()
            .unwrap()
            .sockets
            .values()
            .map(|s| s.try_lock().unwrap().uuid.clone())
            .collect()
    }
    fn get_sockets_count(&self) -> usize {
        self.provide_sockets().try_lock().unwrap().sockets.len()
    }

    fn get_socket_uuid_for_endpoint(
        &self,
        endpoint: Endpoint,
    ) -> Option<ComInterfaceSocketUUID> {
        let sockets = self.provide_sockets();
        let sockets = sockets.try_lock().unwrap();

        sockets
            .sockets
            .values()
            .find(|s| {
                s.try_lock().unwrap().direct_endpoint == Some(endpoint.clone())
            })
            .map(|s| s.try_lock().unwrap().uuid.clone())
    }
    fn get_socket_uuid_at(
        &self,
        index: usize,
    ) -> Option<ComInterfaceSocketUUID> {
        let sockets = self.provide_sockets();
        let sockets = sockets.try_lock().unwrap();

        sockets
            .sockets
            .values()
            .nth(index)
            .map(|s| s.try_lock().unwrap().uuid.clone())
    }
    fn get_socket_at(
        &self,
        index: usize,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        let sockets = self.provide_sockets();
        let sockets = sockets.try_lock().unwrap();

        sockets.sockets.values().nth(index).cloned()
    }

    fn has_socket_with_uuid(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> bool {
        let sockets = self.provide_sockets();
        let sockets = sockets.try_lock().unwrap();
        sockets.sockets.contains_key(&socket_uuid)
    }

    fn get_socket_with_uuid(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        let sockets = self.provide_sockets();
        let sockets = sockets.try_lock().unwrap();

        sockets.sockets.get(&socket_uuid).cloned()
    }
}

pub trait SingleSocketProvider {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSocketManager>>;

    fn get_socket(&self) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        self.provide_sockets()
            .try_lock()
            .unwrap()
            .sockets
            .values()
            .next()
            .cloned()
    }

    fn get_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.get_socket()
            .map(|s| s.try_lock().unwrap().uuid.clone())
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
