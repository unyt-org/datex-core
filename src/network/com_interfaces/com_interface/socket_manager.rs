use crate::collections::HashMap;
use crate::network::com_hub::ComInterfaceImplementationFactoryFn;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::properties::InterfaceDirection;
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
pub struct ComInterfaceSocketManager {
    interface_uuid: ComInterfaceUUID,
    socket_event_sender: UnboundedSender<ComInterfaceSocketEvent>,
}

impl ComInterfaceSocketManager {
    pub fn new_with_sender(
        interface_uuid: ComInterfaceUUID,
        sender: UnboundedSender<ComInterfaceSocketEvent>,
    ) -> Self {
        ComInterfaceSocketManager {
            interface_uuid,
            socket_event_sender: sender,
        }
    }
}

impl ComInterfaceSocketManager {
    /// Adds a new socket with the Open state and notifies listeners on ComHub
    pub fn add_socket(&mut self, socket: ComInterfaceSocket) {
        self.socket_event_sender
            .start_send(ComInterfaceSocketEvent::NewSocket(socket))
            .unwrap();
    }

    /// Removes a socket by its UUID and notifies listeners on ComHub
    pub fn remove_socket(&mut self, socket_uuid: ComInterfaceSocketUUID) {
        self.socket_event_sender
            .start_send(ComInterfaceSocketEvent::RemovedSocket(
                socket_uuid
            ))
            .unwrap();
        // FIXME socket state
        // if let Some(socket) = self.sockets.get(socket_uuid) {
        //     socket.try_lock().unwrap().state = SocketState::Destroyed;
        // }
    }

    /// Registers an endpoint for a socket and notifies listeners on ComHub
    pub fn register_socket_with_endpoint(
        &mut self,
        socket_uuid: ComInterfaceSocketUUID,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), ComInterfaceError> {
        debug!("Socket registered: {socket_uuid} {endpoint}");
        self.socket_event_sender
            .start_send(ComInterfaceSocketEvent::RegisteredSocket(
                socket_uuid,
                distance as i8,
                endpoint,
            ))
            .unwrap();
        Ok(())
    }

    pub fn create_and_init_socket(
        &mut self,
        direction: InterfaceDirection,
        channel_factor: u32,
    ) -> (ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>) {
        let (socket, sender) = ComInterfaceSocket::init(
            self.interface_uuid.clone(),
            direction,
            channel_factor,
        );
        let socket_uuid = socket.uuid.clone();
        self.add_socket(socket);
        (socket_uuid, sender)
    }
}
