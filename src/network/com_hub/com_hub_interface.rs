use crate::collections::{HashMap, HashSet};
use crate::global::protocol_structures::block_header::BlockType;
use crate::global::protocol_structures::routing_header::SignatureType;
use crate::network::com_hub::managers::interface_manager::{
    ComInterfaceFactoryFn, InterfaceManager,
};
use crate::network::com_hub::managers::socket_manager::{
    EndpointIterateOptions, SocketManager,
};
use crate::network::com_hub::options::ComHubOptions;
use crate::std_sync::Mutex;
use crate::stdlib::boxed::Box;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::sync::Arc;
use crate::stdlib::vec;
use crate::stdlib::vec::Vec;
use crate::stdlib::{cell::RefCell, rc::Rc};
use crate::task::spawn_local;
use crate::task::{self, UnboundedReceiver, sleep, spawn_with_panic_notify};
use crate::utils::time::Time;
use core::cmp::PartialEq;
use core::fmt::{Debug, Display, Formatter};
use core::prelude::rust_2024::*;
use core::result::Result;
use core::time::Duration;
use futures::channel::oneshot::Sender;
use itertools::Itertools;
use log::{debug, error, info, warn};
#[cfg(feature = "tokio_runtime")]
use tokio::task::yield_now;
use webrtc::util::vnet::interface;

use crate::values::core_values::endpoint::{Endpoint, EndpointInstance};
use crate::global::dxb_block::{DXBBlock, IncomingSection};
use crate::network::block_handler::{BlockHandler, BlockHistoryData};
use crate::network::com_hub::network_tracing::{NetworkTraceHop, NetworkTraceHopDirection, NetworkTraceHopSocket};
use crate::network::com_interfaces::com_interface::{ComInterface, ComInterfaceEvent, ComInterfaceSocketEvent, ComInterfaceUUID};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, ReconnectionConfig,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::default_com_interfaces::local_loopback_interface::LocalLoopbackInterface;
use crate::runtime::AsyncContext;
use crate::values::value_container::ValueContainer;
use crate::network::com_hub::{
    ComHub, ComHubError, InterfacePriority
};

/// Interface management methods
impl ComHub {
    /// Registers a new interface factory for the given interface type
    pub fn register_interface_factory(
        &self,
        interface_type: String,
        factory: ComInterfaceFactoryFn,
    ) {
        self.interface_manager
            .borrow_mut()
            .register_interface_factory(interface_type, factory);
    }

    /// Adds a new interface to the ComHub
    pub fn add_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        self.interface_manager
            .borrow_mut()
            .add_interface(interface.clone(), priority)?;

        // handle socket events
        self.handle_socket_events(interface.clone());
        // handle interface events
        self.handle_interface_events(interface);
        Ok(())
    }

    /// Internal method to handle interface events
    fn handle_interface_events(
        &self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) {
        let interface_event_receiver =
            interface.borrow_mut().take_interface_event_receiver();
        let uuid = interface.borrow().uuid().clone();
        spawn_with_panic_notify(
            &self.async_context,
            handle_interface_events(
                uuid,
                interface_event_receiver,
                self.interface_manager.clone(),
            ),
        );
    }

    /// Returns the com interface for a given socket UUID
    /// The interface and socket must be registered in the ComHub,
    /// otherwise a panic will be triggered
    pub(crate) fn dyn_interface_for_socket_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Rc<RefCell<dyn ComInterface>> {
        let socket = self.socket_manager.borrow().socket_by_uuid(socket_uuid);
        let socket = socket.try_lock().unwrap();
        self.interface_manager
            .borrow()
            .dyn_interface_by_uuid(&socket.interface_uuid)
    }

    /// Creates a new interface of the given type with the provided setup data
    pub async fn create_interface(
        &self,
        interface_type: &str,
        setup_data: ValueContainer,
        priority: InterfacePriority,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComHubError> {
        self.interface_manager
            .borrow_mut()
            .create_interface(interface_type, setup_data, priority)
            .await
    }
}

async fn handle_interface_events(
    uuid: ComInterfaceUUID,
    mut receiver_queue: UnboundedReceiver<ComInterfaceEvent>,
    interface_manager: Rc<RefCell<InterfaceManager>>,
) {
    while let Some(event) = receiver_queue.next().await {
        interface_manager
            .borrow_mut()
            .handle_interface_event(&uuid, event);
    }
}
