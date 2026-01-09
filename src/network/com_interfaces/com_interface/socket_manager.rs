use crate::collections::HashMap;
use crate::network::com_hub::ComInterfaceImplementationFactoryFn;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::socket::{
    ComInterfaceSocket, ComInterfaceSocketEvent, ComInterfaceSocketUUID,
    SocketState,
};
use crate::stdlib::any::Any;
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::stdlib::sync::{Arc, Mutex};
use crate::task::{
    UnboundedReceiver, UnboundedSender, create_unbounded_channel,
};
use crate::utils::uuid::UUID;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use binrw::error::CustomError;
use core::cell::Cell;
use core::fmt::Display;
use core::pin::Pin;
use core::time::Duration;
use log::debug;

#[derive(Debug)]
pub struct ComInterfaceSockets {
    pub sockets:
        HashMap<ComInterfaceSocketUUID, Arc<Mutex<ComInterfaceSocket>>>,
    socket_event_sender: UnboundedSender<ComInterfaceSocketEvent>,
}

impl ComInterfaceSockets {
    pub fn new_with_sender(
        sender: UnboundedSender<ComInterfaceSocketEvent>,
    ) -> Self {
        ComInterfaceSockets {
            sockets: HashMap::new(),
            socket_event_sender: sender,
        }
    }
}

impl ComInterfaceSockets {
    /// Adds a new socket with the Open state and notifies listeners on ComHub
    pub fn add_socket(&mut self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        {
            let mut socket_mut = socket.try_lock().unwrap();
            let uuid = socket_mut.uuid.clone();
            socket_mut.state = SocketState::Open;
            self.sockets.insert(uuid.clone(), socket.clone());
            debug!("Socket added: {uuid}");
        }
        self.socket_event_sender
            .start_send(ComInterfaceSocketEvent::NewSocket(socket.clone()))
            .unwrap();
    }

    /// Removes a socket by its UUID and notifies listeners on ComHub
    pub fn remove_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        self.sockets.remove(socket_uuid);
        self.socket_event_sender
            .start_send(ComInterfaceSocketEvent::RemovedSocket(
                socket_uuid.clone(),
            ))
            .unwrap();
        if let Some(socket) = self.sockets.get(socket_uuid) {
            socket.try_lock().unwrap().state = SocketState::Destroyed;
        }
    }

    /// Gets a socket by its UUID
    pub fn socket_by_uuid(
        &self,
        uuid: &ComInterfaceSocketUUID,
    ) -> Option<Arc<Mutex<ComInterfaceSocket>>> {
        self.sockets.get(uuid).cloned()
    }

    /// Registers an endpoint for a socket and notifies listeners on ComHub
    pub fn register_socket_endpoint(
        &mut self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), ComInterfaceError> {
        let socket = self.sockets.get(&socket_uuid);
        if socket.is_none() {
            return Err(ComInterfaceError::SocketNotFound);
        }
        {
            let mut socket = socket.unwrap().try_lock().unwrap();
            if socket.direct_endpoint.is_none() {
                socket.direct_endpoint = Some(endpoint.clone());
            }
        }

        debug!("Socket registered: {socket_uuid} {endpoint}");
        self.socket_event_sender
            .start_send(ComInterfaceSocketEvent::RegisteredSocket(
                socket_uuid,
                distance as i8,
                endpoint.clone(),
            ))
            .unwrap();
        Ok(())
    }
}
