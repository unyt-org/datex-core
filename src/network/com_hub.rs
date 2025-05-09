use crate::global::protocol_structures::block_header::BlockType;
use crate::global::protocol_structures::routing_header::{
    ReceiverEndpoints, SignatureType,
};
use crate::runtime::global_context::get_global_context;
use crate::stdlib::{cell::RefCell, rc::Rc};
use crate::task::{sleep, spawn_with_panic_notify};
use futures_util::future::join_all;

use futures::FutureExt;
use itertools::Itertools;
use log::{debug, error, info, warn};
use std::any::Any;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(feature = "tokio_runtime")]
use tokio::task::yield_now;
// FIXME no-std

use super::com_interfaces::com_interface::{
    self, ComInterfaceError, ComInterfaceState
};
use super::com_interfaces::{
    com_interface::ComInterface, com_interface_socket::ComInterfaceSocket,
};
use crate::datex_values::{Endpoint, EndpointInstance};
use crate::global::dxb_block::{DXBBlock, IncomingSection};
use crate::network::block_handler::{BlockHandler};
use crate::network::com_hub_network_tracing::{NetworkTraceHop, NetworkTraceHopDirection, NetworkTraceHopSocket};
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties, ReconnectionConfig,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::default_com_interfaces::local_loopback_interface::LocalLoopbackInterface;

#[derive(Debug, Clone)]
pub struct DynamicEndpointProperties {
    pub known_since: u64,
    pub distance: u8,
    pub is_direct: bool,
    pub channel_factor: u32,
    pub direction: InterfaceDirection,
}

pub type ComInterfaceFactoryFn =
    fn(
        setup_data: Box<dyn Any>,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComInterfaceError>;

pub struct ComHub {
    /// the runtime endpoint of the hub (@me)
    pub endpoint: Endpoint,

    /// a list of all available interface factories, keyed by their interface type
    pub interface_factories: RefCell<HashMap<String, ComInterfaceFactoryFn>>,

    /// a list of all available interfaces, keyed by their UUID
    pub interfaces: RefCell<
        HashMap<
            ComInterfaceUUID,
            (Rc<RefCell<dyn ComInterface>>, InterfacePriority),
        >,
    >,

    /// a list of all available sockets, keyed by their UUID
    /// contains the socket itself and a list of endpoints currently associated with it
    pub sockets: RefCell<
        HashMap<
            ComInterfaceSocketUUID,
            (Arc<Mutex<ComInterfaceSocket>>, HashSet<Endpoint>),
        >,
    >,

    /// a blacklist of sockets that are not allowed to be used for a specific endpoint
    pub endpoint_sockets_blacklist:
        RefCell<HashMap<Endpoint, HashSet<ComInterfaceSocketUUID>>>,

    /// fallback sockets that are used if no direct endpoint reachable socket is available
    /// sorted by priority
    pub fallback_sockets: RefCell<Vec<(ComInterfaceSocketUUID, u16)>>,

    /// a list of all available sockets for each endpoint, with additional
    /// DynamicEndpointProperties metadata
    pub endpoint_sockets: RefCell<
        HashMap<
            Endpoint,
            Vec<(ComInterfaceSocketUUID, DynamicEndpointProperties)>,
        >,
    >,

    pub block_handler: BlockHandler,
}

#[derive(Debug, Clone, Default)]
struct EndpointIterateOptions<'a> {
    pub only_direct: bool,
    pub exact_instance: bool,
    pub exclude_sockets: &'a [ComInterfaceSocketUUID],
}

impl Default for ComHub {
    fn default() -> Self {
        ComHub {
            endpoint: Endpoint::default(),
            interface_factories: RefCell::new(HashMap::new()),
            interfaces: RefCell::new(HashMap::new()),
            endpoint_sockets: RefCell::new(HashMap::new()),
            block_handler: BlockHandler::new(),
            sockets: RefCell::new(HashMap::new()),
            fallback_sockets: RefCell::new(Vec::new()),
            endpoint_sockets_blacklist: RefCell::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum InterfacePriority {
    /// The interface will not be used for fallback routing if no other interface is available
    /// This is useful for interfaces which cannot communicate with the outside world or are not
    /// capable of redirecting large amounts of data
    None,
    /// The interface will be used for fallback routing if no other interface is available,
    /// depending on the defined priority
    /// A higher number means a higher priority
    Priority(u16),
}

impl Default for InterfacePriority {
    fn default() -> Self {
        InterfacePriority::Priority(0)
    }
}

#[derive(Debug)]
pub enum ComHubError {
    InterfaceError(ComInterfaceError),
    InterfaceCloseFailed,
    InterfaceNotConnected,
    InterfaceDoesNotExist,
    InterfaceAlreadyExists,
    InterfaceTypeDoesNotExist,
    NoResponse,
}

#[derive(Debug)]
pub enum SocketEndpointRegistrationError {
    SocketDisconnected,
    SocketUninitialized,
    SocketEndpointAlreadyRegistered,
}

impl ComHub {
    pub fn new(endpoint: impl Into<Endpoint>) -> ComHub {
        ComHub {
            endpoint: endpoint.into(),
            ..ComHub::default()
        }
    }

    pub async fn init(&self) -> Result<(), ComHubError> {
        // add default local loopback interface
        let local_interface = LocalLoopbackInterface::new();
        self.open_and_add_interface(
            Rc::new(RefCell::new(local_interface)),
            InterfacePriority::None,
        )
        .await
    }

    /// Registers a new interface factory for a specific interface implementation.
    /// This allows the ComHub to create new instances of the interface on demand.
    pub fn register_interface_factory(
        &self,
        interface_type: String,
        factory: ComInterfaceFactoryFn,
    ) {
        self.interface_factories
            .borrow_mut()
            .insert(interface_type, factory);
    }

    /// Creates a new interface instance using the registered factory
    /// for the specified interface type if it exists.
    /// The interface is opened and added to the ComHub.
    pub async fn create_interface(
        &self,
        interface_type: &str,
        setup_data: Box<dyn Any>,
        priority: InterfacePriority,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComHubError> {
        info!("creating interface {interface_type}");
        if let Some(factory) =
            self.interface_factories.borrow().get(interface_type)
        {
            let interface =
                factory(setup_data).map_err(ComHubError::InterfaceError)?;
            let uuid = interface.borrow().get_uuid().clone();
            let res = self
                .open_and_add_interface(interface.clone(), priority)
                .await
                .map(|_| interface);
            res
        } else {
            Err(ComHubError::InterfaceTypeDoesNotExist)
        }
    }

    fn try_downcast<T: 'static>(
        input: Rc<RefCell<dyn ComInterface>>,
    ) -> Option<Rc<RefCell<T>>> {
        // Try to get a reference to the inner value
        if input.borrow().as_any().is::<T>() {
            // SAFETY: We're ensuring T is the correct type via the check
            let ptr = Rc::into_raw(input) as *const RefCell<T>;
            unsafe { Some(Rc::from_raw(ptr)) }
        } else {
            None
        }
    }

    pub fn get_interface_by_uuid<T: ComInterface>(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Option<Rc<RefCell<T>>> {
        ComHub::try_downcast(
            self.interfaces.borrow().get(interface_uuid)?.0.clone(),
        )
    }

    pub fn has_interface(&self, interface_uuid: &ComInterfaceUUID) -> bool {
        self.interfaces.borrow().contains_key(interface_uuid)
    }

    pub fn get_interface_ref_by_uuid(
        &self,
        uuid: &ComInterfaceUUID,
    ) -> Option<Rc<RefCell<dyn ComInterface>>> {
        self.interfaces
            .borrow()
            .get(uuid)
            .map(|(interface, _)| interface.clone())
    }

    pub async fn open_and_add_interface(
        &self,
        interface: Rc<RefCell<dyn ComInterface>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        if interface.borrow().get_state() != ComInterfaceState::Connected {
            // If interface is not connected, open it
            // and wait for it to be connected
            interface.borrow_mut().handle_open().await;
        }
        self.add_interface(interface.clone(), priority)
    }

    pub fn add_interface(
        &self,
        interface: Rc<RefCell<dyn ComInterface>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        let uuid = interface.borrow().get_uuid().clone();
        let mut interfaces = self.interfaces.borrow_mut();
        if interfaces.contains_key(&uuid) {
            return Err(ComHubError::InterfaceAlreadyExists);
        }
        interfaces.insert(uuid, (interface, priority));
        Ok(())
    }

    /// User can proactively remove an interface from the hub.
    /// This will destroy the interface and it's sockets (perform deep cleanup)
    pub async fn remove_interface(
        &self,
        interface_uuid: ComInterfaceUUID,
    ) -> Result<(), ComHubError> {
        info!("Removing interface {interface_uuid}");
        let interface = self
            .interfaces
            .borrow_mut()
            .get_mut(&interface_uuid.clone())
            .ok_or(ComHubError::InterfaceDoesNotExist)?
            .0
            .clone();
        {
            // Async close the interface (stop tasks, server, cleanup internal data)
            let interface = interface.clone();
            let mut interface = interface.borrow_mut();
            interface.handle_destroy().await;
        }

        // Remove old sockets from ComHub that have been deleted by the interface destroy_sockets()
        self.update_sockets();

        self.cleanup_interface(interface_uuid)
            .ok_or(ComHubError::InterfaceDoesNotExist)?;

        Ok(())
    }

    /// The internal cleanup function that removes the interface from the hub
    /// and disabled the default interface if it was set to this interface
    fn cleanup_interface(
        &self,
        interface_uuid: ComInterfaceUUID,
    ) -> Option<Rc<RefCell<dyn ComInterface>>> {
        let interface = self
            .interfaces
            .borrow_mut()
            .remove(&interface_uuid)
            .or(None)?
            .0;
        Some(interface)
    }

    pub(crate) fn receive_block(
        &self,
        block: &DXBBlock,
        socket_uuid: ComInterfaceSocketUUID,
    ) {
        info!("{} received block: {}", self.endpoint, block);

        // ignore invalid blocks (e.g. invalid signature)
        if !self.validate_block(block) {
            warn!("Block validation failed. Dropping block...");
            return;
        }

        let block_type = block.block_header.flags_and_timestamp.block_type();

        // register in block history
        let is_new_block = !self.block_handler.is_block_in_history(block);
        // assign endpoint to socket if none is assigned
        // only if a new block and the sender in not the local endpoint
        if is_new_block && block.routing_header.sender != self.endpoint {
            self.register_socket_endpoint_from_incoming_block(
                socket_uuid.clone(),
                block,
            );
        }

        if !is_new_block {
            // block already in history, ignore
            warn!("Block already in history. Ignoring...");
        }

        if let Some(receivers) = &block.routing_header.receivers.endpoints {
            let is_for_own = receivers.endpoints.iter().any(|e| {
                e == &self.endpoint
                    || e == &Endpoint::ANY
                    || e == &Endpoint::ANY_ALL_INSTANCES
            });

            // handle blocks for own endpoint
            if is_for_own && block_type != BlockType::Hello {
                info!("Block is for this endpoint");

                match block_type {
                    BlockType::Trace => {
                        self.handle_trace_block(block, socket_uuid.clone());
                    }
                    BlockType::TraceBack => {
                        self.handle_trace_back_block(
                            block,
                            socket_uuid.clone(),
                        );
                    }
                    _ => {
                        self.block_handler.handle_incoming_block(block.clone());
                    }
                };
            }

            let should_relay =
                // don't relay "Hello" blocks sent to own endpoint
                !(
                    is_for_own && block_type == BlockType::Hello
                );

            // relay the block to other endpoints
            if should_relay {
                // get all receivers that the block must be relayed to
                let remaining_receivers = if is_for_own {
                    &self.get_remote_receivers(receivers)
                } else {
                    &receivers.endpoints
                };

                // relay the block to all receivers
                if !remaining_receivers.is_empty() {
                    match block_type {
                        BlockType::Trace | BlockType::TraceBack => {
                            self.redirect_trace_block(
                                block,
                                remaining_receivers,
                                socket_uuid.clone(),
                            );
                        }
                        _ => {
                            self.redirect_block(
                                block.clone(),
                                remaining_receivers,
                                socket_uuid.clone(),
                            );
                        }
                    }
                }
            }
        }

        // add to block history
        if is_new_block {
            self.block_handler.add_block_to_history(block, socket_uuid);
        }
    }

    /// Returns a list of all receivers from a given ReceiverEndpoints
    /// excluding the local endpoint
    fn get_remote_receivers(
        &self,
        receiver_endpoints: &ReceiverEndpoints,
    ) -> Vec<Endpoint> {
        receiver_endpoints
            .endpoints
            .iter()
            .filter(|e| e != &&self.endpoint)
            .cloned()
            .collect::<Vec<_>>()
    }

    /// Registers the socket endpoint from an incoming block
    /// if the endpoint is not already registered for the socket
    fn register_socket_endpoint_from_incoming_block(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
        block: &DXBBlock,
    ) {
        let socket = self.get_socket_by_uuid(&socket_uuid);
        let mut socket_ref = socket.lock().unwrap();

        let distance = block.routing_header.distance;
        let sender = block.routing_header.sender.clone();

        // set as direct endpoint if distance = 0
        if socket_ref.direct_endpoint.is_none() && distance == 1 {
            info!(
                "Setting direct endpoint for socket {}: {}",
                socket_ref.uuid, sender
            );
            socket_ref.direct_endpoint = Some(sender.clone());
        }

        drop(socket_ref);
        match self.register_socket_endpoint(
            socket.clone(),
            sender.clone(),
            distance,
        ) {
            Err(SocketEndpointRegistrationError::SocketEndpointAlreadyRegistered) => {
                debug!(
                    "Socket already registered for endpoint {sender}",
                );
            }
            Err(error) => {
                panic!("Failed to register socket endpoint {sender}: {error:?}");
            },
            Ok(_) => { }
        }
    }

    /// Prepares a block and relays it to the given receivers.
    /// The routing distance is incremented by 1.
    pub(crate) fn redirect_block(
        &self,
        block: DXBBlock,
        receivers: &[Endpoint],
        incoming_socket: ComInterfaceSocketUUID,
    ) {
        // check if block has already passed this endpoint (-> bounced back block)
        // and add to blacklist for all receiver endpoints
        let history_block_data =
            self.block_handler.get_block_data_from_history(&block);
        if history_block_data.is_some() {
            for receiver in receivers {
                if receiver != &self.endpoint {
                    info!(
                        "{}: Adding socket {} to blacklist for receiver {}",
                        self.endpoint, incoming_socket, receiver
                    );
                    self.endpoint_sockets_blacklist
                        .borrow_mut()
                        .entry(receiver.clone())
                        .or_default()
                        .insert(incoming_socket.clone());
                }
            }
        }

        let mut block = block.clone();
        block.set_receivers(receivers);
        // increment distance for next hop
        block.routing_header.distance += 1;
        let res = self.send_block(block.clone(), Some(incoming_socket.clone()));

        // send block for unreachable endpoints back to the sender
        if let Err(unreachable_endpoints) = res {
            // try to send back to original socket
            // if already in history, get original socket from history
            // otherwise, directly send back to the incoming socket
            let original_socket =
                if let Some(history_block_data) = history_block_data {
                    history_block_data.original_socket_uuid
                } else {
                    incoming_socket
                };

            let socket_endpoint = self
                .get_socket_by_uuid(&original_socket)
                .lock()
                .unwrap()
                .direct_endpoint
                .clone();

            info!(
                "Sending block for {} back to original socket: {} ({})",
                unreachable_endpoints
                    .iter()
                    .map(|e| e.to_string())
                    .join(","),
                original_socket,
                socket_endpoint
                    .as_ref()
                    .map(|e| e.to_string())
                    .unwrap_or("Unknown".to_string())
            );
            // decrement distance because we are going back
            if block.routing_header.distance <= 1 {
                block.routing_header.distance -= 1;
                //panic!("Distance for redirect block is <= 1. Cannot decrement.");
            } else {
                block.routing_header.distance -= 2;
            }
            self.send_block_addressed(
                block,
                &original_socket,
                &unreachable_endpoints,
            )
        }
    }

    fn validate_block(&self, block: &DXBBlock) -> bool {
        // TODO check for creation time, withdraw if too old (TBD) or in the future

        let is_signed =
            block.routing_header.flags.signature_type() != SignatureType::None;

        match is_signed {
            true => {
                // TODO: verify signature and abort if invalid
                // Check if signature is following in some later block and add them to
                // a pool of incoming blocks awaiting some signature
                true
            }
            false => {
                let endpoint = block.routing_header.sender.clone();
                let is_trusted = {
                    cfg_if::cfg_if! {
                        if #[cfg(feature = "debug")] {
                            get_global_context().debug_flags.allow_unsigned_blocks
                        }
                        else {
                            // TODO Check if the sender is trusted (endpoint + interface) connection
                            false
                        }
                    }
                };
                match is_trusted {
                    true => true,
                    false => {
                        warn!("Block received by {endpoint} is not signed. Dropping block...");
                        false
                    }
                }
            }
        }
    }

    /// Registers a new endpoint that is reachable over the socket if the socket is not
    /// already registered, it will be added to the socket list.
    /// If the provided endpoint is not the same as the socket endpoint, it is registered
    /// as an indirect socket to the endpoint
    pub fn register_socket_endpoint(
        &self,
        socket: Arc<Mutex<ComInterfaceSocket>>,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), SocketEndpointRegistrationError> {
        log::info!(
            "{} registering endpoint {} for socket {}",
            self.endpoint,
            endpoint,
            socket.lock().unwrap().uuid
        );
        let socket_ref = socket.lock().unwrap();

        // if the registered endpoint is the same as the socket endpoint,
        // this is a direct socket to the endpoint
        let is_direct = socket_ref.direct_endpoint == Some(endpoint.clone());

        // cannot register endpoint if socket is not connected
        if !socket_ref.state.is_open() {
            return Err(SocketEndpointRegistrationError::SocketDisconnected);
        }

        // check if the socket is already registered for the endpoint
        if let Some(entries) = self.endpoint_sockets.borrow().get(&endpoint)
            && entries
                .iter()
                .any(|(socket_uuid, _)| socket_uuid == &socket_ref.uuid)
        {
            return Err(SocketEndpointRegistrationError::SocketEndpointAlreadyRegistered);
        }

        let socket_uuid = socket_ref.uuid.clone();
        let channel_factor = socket_ref.channel_factor;
        let direction = socket_ref.direction.clone();
        drop(socket_ref);

        // add endpoint to socket endpoint list
        self.add_socket_endpoint(&socket_uuid, endpoint.clone());

        // add socket to endpoint socket list
        self.add_endpoint_socket(
            &endpoint,
            socket_uuid,
            distance,
            is_direct,
            channel_factor,
            direction,
        );

        // resort sockets for endpoint
        self.sort_sockets(&endpoint);

        Ok(())
    }

    /// Waits for all background tasks scheduled by the update() function to finish
    /// This includes block flushes from `flush_outgoing_blocks()`
    /// and interface (re)-connections from `update_interfaces()`
    pub async fn wait_for_update_async(&self) {
        loop {
            let mut is_done = true;
            for interface in self.interfaces.borrow().values() {
                let interface = interface.0.clone();
                let interface = interface.borrow_mut();
                let outgoing_blocks_count =
                    interface.get_info().outgoing_blocks_count.get();
                // blocks are still sent out on this interface
                if outgoing_blocks_count > 0 {
                    is_done = false;
                    break;
                }
                // interface is still in connection task
                if interface.get_state() == ComInterfaceState::Connecting {
                    is_done = false;
                    break;
                }
            }
            if is_done {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    }

    /// Updates all sockets and interfaces,
    /// collecting incoming data and sending out queued blocks.
    /// In contrast to the update() function, this function is asynchronous
    /// and will wait for all background tasks scheduled by the update() function to finish
    pub async fn update_async(&self) {
        self.update();
        self.wait_for_update_async().await;
    }

    /// Adds a socket to the socket list for a specific endpoint,
    /// attaching metadata as DynamicEndpointProperties
    fn add_endpoint_socket(
        &self,
        endpoint: &Endpoint,
        socket_uuid: ComInterfaceSocketUUID,
        distance: u8,
        is_direct: bool,
        channel_factor: u32,
        direction: InterfaceDirection,
    ) {
        let mut endpoint_sockets = self.endpoint_sockets.borrow_mut();
        if !endpoint_sockets.contains_key(endpoint) {
            endpoint_sockets.insert(endpoint.clone(), Vec::new());
        }

        let endpoint_sockets = endpoint_sockets.get_mut(endpoint).unwrap();
        endpoint_sockets.push((
            socket_uuid,
            DynamicEndpointProperties {
                known_since: get_global_context().time.lock().unwrap().now(),
                distance,
                is_direct,
                channel_factor,
                direction,
            },
        ));
    }

    /// Adds a socket to the socket list.
    /// If the priority is not set to `InterfacePriority::None`, the socket
    /// is also registered as a fallback socket for outgoing connections with the
    /// specified priority.
    fn add_socket(
        &self,
        socket: Arc<Mutex<ComInterfaceSocket>>,
        priority: InterfacePriority,
    ) {
        let socket_ref = socket.lock().unwrap();
        let socket_uuid = socket_ref.uuid.clone();
        if self.sockets.borrow().contains_key(&socket_ref.uuid) {
            panic!("Socket {} already exists in ComHub", socket_ref.uuid);
        }

        // info!(
        //     "Adding socket {} to ComHub with priority {:?}",
        //     socket_ref.uuid, priority
        // );

        if !socket_ref.can_send() && priority != InterfacePriority::None {
            panic!(
                "Socket {} cannot be used for fallback routing, since it has no send capability",
                socket_ref.uuid
            );
        }

        self.sockets
            .borrow_mut()
            .insert(socket_ref.uuid.clone(), (socket.clone(), HashSet::new()));

        // add outgoing socket to fallback sockets list if they have a priority flag
        if socket_ref.can_send() {
            match priority {
                InterfacePriority::None => {
                    // do nothing
                }
                InterfacePriority::Priority(priority) => {
                    // add socket to fallback sockets list
                    self.add_fallback_socket(&socket_uuid, priority);
                }
            }
        }

        // send empty block to socket to say hello
        let mut block: DXBBlock = DXBBlock::default();
        block
            .block_header
            .flags_and_timestamp
            .set_block_type(BlockType::Hello);
        block
            .routing_header
            .flags
            .set_signature_type(SignatureType::Unencrypted);
        // TODO include fingerprint of the own public key into body

        let block = self.prepare_own_block(block);

        drop(socket_ref);
        self.send_block_addressed(block, &socket_uuid, &[Endpoint::ANY]);
    }

    /// Registers a socket as a fallback socket for outgoing connections
    /// that can be used if no known route exists for an endpoint
    /// Note: only sockets that support sending data should be used as fallback sockets
    fn add_fallback_socket(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
        priority: u16,
    ) {
        // add to vec
        let mut fallback_sockets = self.fallback_sockets.borrow_mut();
        fallback_sockets.push((socket_uuid.clone(), priority));
        // sort_by priority
        fallback_sockets.sort_by(|(_, a), (_, b)| b.cmp(a));
    }

    /// Removes a socket from the socket list
    fn delete_socket(&self, socket_uuid: &ComInterfaceSocketUUID) {
        self.sockets
            .borrow_mut()
            .remove(socket_uuid)
            .or_else(|| panic!("Socket {socket_uuid} not found in ComHub"));

        // remove socket from endpoint socket list
        // remove endpoint key from endpoint_sockets if not sockets present
        self.endpoint_sockets.borrow_mut().retain(|_, sockets| {
            sockets.retain(|(uuid, _)| uuid != socket_uuid);
            !sockets.is_empty()
        });

        // remove socket if it is the default socket
        self.fallback_sockets
            .borrow_mut()
            .retain(|(uuid, _)| uuid != socket_uuid);
    }

    /// Adds an endpoint to the endpoint list of a specific socket
    fn add_socket_endpoint(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
        endpoint: Endpoint,
    ) {
        assert!(
            self.sockets.borrow().contains_key(socket_uuid),
            "Socket not found in ComHub"
        );
        // add endpoint to socket endpoint list
        self.sockets
            .borrow_mut()
            .get_mut(socket_uuid)
            .unwrap()
            .1
            .insert(endpoint.clone());
    }

    /// Sorts the sockets for an endpoint:
    /// - socket with send capability first
    /// - then direct sockets
    /// - then sort by channel channel_factor (latency, bandwidth)
    /// - then sort by socket connect_timestamp
    /// When the global debug flag `enable_deterministic_behavior` is set,
    /// Sockets are not sorted by their connect_timestamp to make sure that the order of
    /// received blocks has no effect on the routing priorities
    fn sort_sockets(&self, endpoint: &Endpoint) {
        let mut endpoint_sockets = self.endpoint_sockets.borrow_mut();
        let sockets = endpoint_sockets.get_mut(endpoint).unwrap();

        sockets.sort_by(|(_, a), (_, b)| {
            // sort by channel_factor
            b.is_direct
                .cmp(&a.is_direct)
                .then_with(|| b.channel_factor.cmp(&a.channel_factor))
                .then_with(|| b.distance.cmp(&a.distance))
                .then_with(
                    || {
                        cfg_if::cfg_if! {
                            if #[cfg(feature = "debug")] {
                                if get_global_context().debug_flags.enable_deterministic_behavior {
                                    Ordering::Equal
                                }
                                else {
                                    b.known_since.cmp(&a.known_since)
                                }
                            }
                            else {
                                return b.known_since.cmp(&a.known_since)
                            }
                        }
                    }
                )

        });
    }

    /// Returns the socket for a given UUID
    /// The socket must be registered in the ComHub,
    /// otherwise a panic will be triggered
    pub(crate) fn get_socket_by_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Arc<Mutex<ComInterfaceSocket>> {
        self.sockets
            .borrow()
            .get(socket_uuid)
            .map(|socket| socket.0.clone())
            .unwrap_or_else(|| {
                panic!("Socket for uuid {socket_uuid} not found")
            })
    }

    /// Returns the com interface for a given UUID
    /// The interface must be registered in the ComHub,
    /// otherwise a panic will be triggered
    pub(crate) fn get_com_interface_by_uuid(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Rc<RefCell<dyn ComInterface>> {
        self.interfaces
            .borrow()
            .get(interface_uuid)
            .unwrap_or_else(|| {
                panic!("Interface for uuid {interface_uuid} not found")
            })
            .0
            .clone()
    }

    /// Returns the com interface for a given socket UUID
    /// The interface and socket must be registered in the ComHub,
    /// otherwise a panic will be triggered
    pub(crate) fn get_com_interface_from_socket_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Rc<RefCell<dyn ComInterface>> {
        let socket = self.get_socket_by_uuid(socket_uuid);
        let socket = socket.lock().unwrap();
        self.get_com_interface_by_uuid(&socket.interface_uuid)
    }

    /// Returns an iterator over all sockets for a given endpoint
    /// The sockets are yielded in the order of their priority, starting with the
    /// highest priority socket (the best socket for sending data to the endpoint)
    fn iterate_endpoint_sockets<'a>(
        &'a self,
        endpoint: &'a Endpoint,
        options: EndpointIterateOptions<'a>,
    ) -> impl Iterator<Item = ComInterfaceSocketUUID> + 'a {
        std::iter::from_coroutine(
            #[coroutine]
            move || {
                let endpoint_sockets_borrow = self.endpoint_sockets.borrow();
                // TODO: can we optimize this to avoid cloning the endpoint_sockets vector?
                let endpoint_sockets =
                    endpoint_sockets_borrow.get(endpoint).cloned();
                if endpoint_sockets.is_none() {
                    return;
                }
                for (socket_uuid, _) in endpoint_sockets.unwrap() {
                    {
                        let socket = self.get_socket_by_uuid(&socket_uuid);
                        let socket = socket.lock().unwrap();

                        // check if only_direct is set and the endpoint equals the direct endpoint of the socket
                        if options.only_direct
                            && socket.direct_endpoint.is_some()
                            && socket.direct_endpoint.as_ref().unwrap()
                                == endpoint
                        {
                            debug!(
                                "No direct socket found for endpoint {endpoint}. Skipping..."
                            );
                            continue;
                        }

                        // check if the socket is excluded if exclude_socket is set
                        if options.exclude_sockets.contains(&socket.uuid) {
                            debug!(
                                    "Socket {} is excluded for endpoint {}. Skipping...",
                                    socket.uuid,
                                    endpoint
                                );
                            continue;
                        }

                        // only yield outgoing sockets
                        // if a non-outgoing socket is found, all following sockets
                        // will also be non-outgoing
                        if !socket.can_send() {
                            info!(
                                "Socket {} is not outgoing for endpoint {}. Skipping...",
                                socket.uuid,
                                endpoint
                            );
                            return;
                        }
                    }

                    debug!(
                        "Found matching socket {socket_uuid} for endpoint {endpoint}"
                    );
                    yield socket_uuid.clone()
                }
            },
        )
    }

    /// Finds the best matching socket over which an endpoint is known to be reachable.
    fn find_known_endpoint_socket(
        &self,
        endpoint: &Endpoint,
        exclude_socket: &[ComInterfaceSocketUUID],
    ) -> Option<ComInterfaceSocketUUID> {
        match endpoint.instance {
            // find socket for any endpoint instance
            EndpointInstance::Any => {
                let options = EndpointIterateOptions {
                    only_direct: false,
                    exact_instance: false,
                    exclude_sockets: exclude_socket,
                };
                if let Some(socket) =
                    self.iterate_endpoint_sockets(endpoint, options).next()
                {
                    return Some(socket);
                }
                None
            }

            // find socket for exact instance
            EndpointInstance::Instance(_) => {
                // iterate over all sockets of all interfaces
                let options = EndpointIterateOptions {
                    only_direct: false,
                    exact_instance: true,
                    exclude_sockets: exclude_socket,
                };
                if let Some(socket) =
                    self.iterate_endpoint_sockets(endpoint, options).next()
                {
                    return Some(socket);
                }
                None
            }

            // TODO: how to handle broadcasts?
            EndpointInstance::All => {
                todo!()
            }
        }
    }

    /// Finds the best socket over which to send a block to an endpoint.
    /// If a known socket is found, it is returned, otherwise the default socket is returned, if it
    /// exists and is not excluded.
    fn find_best_endpoint_socket(
        &self,
        endpoint: &Endpoint,
        exclude_sockets: &[ComInterfaceSocketUUID],
    ) -> Option<ComInterfaceSocketUUID> {
        // if the endpoint is the same as the hub endpoint, try to find an interface
        // that redirects @@local
        if endpoint == &self.endpoint
            && let Some(socket) = self
                .find_known_endpoint_socket(&Endpoint::LOCAL, exclude_sockets)
        {
            return Some(socket);
        }

        // find best known socket for endpoint
        let matching_socket =
            self.find_known_endpoint_socket(endpoint, exclude_sockets);

        // if a matching socket is found, return it
        if matching_socket.is_some() {
            matching_socket
        }
        // otherwise, return the highest priority socket that is not excluded
        else {
            let sockets = self.fallback_sockets.borrow();
            for (socket_uuid, _) in sockets.iter() {
                let socket = self.get_socket_by_uuid(socket_uuid);
                info!(
                    "{}: Find best for {}: {} ({}); excluded:{}",
                    self.endpoint,
                    endpoint,
                    socket_uuid,
                    socket
                        .lock()
                        .unwrap()
                        .direct_endpoint
                        .clone()
                        .map(|e| e.to_string())
                        .unwrap_or("None".to_string()),
                    exclude_sockets.contains(socket_uuid)
                );
                if !exclude_sockets.contains(socket_uuid) {
                    return Some(socket_uuid.clone());
                }
            }
            None
        }
    }

    /// Returns all receivers to which the block has to be sent, grouped by the
    /// outbound socket uuids
    fn get_outbound_receiver_groups(
        &self,
        block: &DXBBlock,
        incoming_socket: Option<ComInterfaceSocketUUID>,
    ) -> Option<Vec<(Option<ComInterfaceSocketUUID>, Vec<Endpoint>)>> {
        if let Some(receivers) = block.receivers() {
            if !receivers.is_empty() {
                let endpoint_sockets = receivers
                    .iter()
                    .map(|e| {
                        let mut exclude_sockets = vec![];
                        if let Some(original_socket) = &incoming_socket {
                            exclude_sockets.push(original_socket.clone());
                        }
                        // add sockets from endpoint blacklist
                        if let Some(blacklist) =
                            self.endpoint_sockets_blacklist.borrow().get(e)
                        {
                            exclude_sockets.extend(blacklist.iter().cloned());
                        }
                        let socket =
                            self.find_best_endpoint_socket(e, &exclude_sockets);
                        (socket, e)
                    })
                    .group_by(|(socket, _)| socket.clone())
                    .into_iter()
                    .map(|(socket, group)| {
                        let endpoints = group
                            .map(|(_, endpoint)| endpoint.clone())
                            .collect::<Vec<_>>();
                        (socket, endpoints)
                    })
                    .collect::<Vec<_>>();

                Some(endpoint_sockets)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Runs the update loop for the ComHub.
    /// This method will continuously handle incoming data, send out
    /// queued blocks and update the sockets.
    pub fn start_update_loop(self_rc: Rc<Self>) {
        spawn_with_panic_notify(async move {
            loop {
                self_rc.update();
                sleep(Duration::from_millis(1)).await;
            }
        });
    }

    /// Update all sockets and interfaces,
    /// collecting incoming data and sending out queued blocks.
    /// Updates are scheduled in local tasks and are not immediately visible.
    /// To wait for the block update to finish, use `wait_for_update_async()`.
    fn update(&self) {
        // update all interfaces
        self.update_interfaces();

        // update own socket lists for routing
        self.update_sockets();

        // update sockets block collectors
        self.collect_incoming_data();

        // receive blocks from all sockets
        self.receive_incoming_blocks();

        // send all queued blocks from all interfaces
        self.flush_outgoing_blocks();
    }

    /// Prepares a block for sending out by updating the creation timestamp,
    /// sender and add signature and encryption if needed.
    fn prepare_own_block(&self, mut block: DXBBlock) -> DXBBlock {
        // TODO signature & encryption
        let now = get_global_context().clone().time.lock().unwrap().now();
        block.routing_header.sender = self.endpoint.clone();
        block
            .block_header
            .flags_and_timestamp
            .set_creation_timestamp(now);

        // set distance to 1
        block.routing_header.distance = 1;

        block
    }

    /// Public method to send an outgoing block from this endpoint. Called by the runtime.
    pub fn send_own_block(&self, mut block: DXBBlock) {
        block = self.prepare_own_block(block);
        self.send_block(block, None);
    }

    /// Sends a block and wait for a response block.
    pub async fn send_own_block_await_response(
        &self,
        block: DXBBlock,
    ) -> Result<IncomingSection, ComHubError> {
        let scope_id = block.block_header.scope_id;
        let block_index = block.block_header.section_index;
        {
            self.send_own_block(block);
        }
        // yield
        #[cfg(feature = "tokio_runtime")]
        yield_now().await;

        let res = self
            .block_handler
            .wait_for_incoming_response_block(scope_id, block_index)
            .await
            .ok_or(ComHubError::NoResponse);

        res
    }

    /// Sends a block to all endpoints specified in the block header.
    /// The routing algorithm decides which sockets are used to send the block, based on the endpoint.
    /// A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
    /// The original_socket parameter is used to prevent sending the block back to the sender.
    /// When this method is called, the block is queued in the send queue.
    /// Returns an Err with a list of unreachable endpoints if the block could not be sent to all endpoints.
    pub fn send_block(
        &self,
        block: DXBBlock,
        incoming_socket: Option<ComInterfaceSocketUUID>,
    ) -> Result<(), Vec<Endpoint>> {
        let outbound_receiver_groups =
            self.get_outbound_receiver_groups(&block, incoming_socket);

        if outbound_receiver_groups.is_none() {
            error!("No outbound receiver groups found for block");
            return Err(vec![]);
        }

        let outbound_receiver_groups = outbound_receiver_groups.unwrap();

        let mut unreachable_endpoints = vec![];

        for (receiver_socket, endpoints) in outbound_receiver_groups {
            if let Some(socket_uuid) = receiver_socket {
                self.send_block_addressed(
                    block.clone(),
                    &socket_uuid,
                    &endpoints,
                );
            } else {
                error!("{}: cannot send block, no receiver sockets found for endpoints {:?}", self.endpoint, endpoints.iter().map(|e| e.to_string()).collect::<Vec<_>>());
                unreachable_endpoints.extend(endpoints);
            }
        }

        if !unreachable_endpoints.is_empty() {
            return Err(unreachable_endpoints);
        }
        Ok(())
    }

    /// Sends a block via a socket to a list of endpoints.
    /// Before the block is sent, it is modified to include the list of endpoints as receivers.
    fn send_block_addressed(
        &self,
        mut block: DXBBlock,
        socket_uuid: &ComInterfaceSocketUUID,
        endpoints: &[Endpoint],
    ) {
        block.set_receivers(endpoints);

        // if type is Trace or TraceBack, add the outgoing socket to the hops
        match block.block_header.flags_and_timestamp.block_type() {
            BlockType::Trace | BlockType::TraceBack => {
                let distance = block.routing_header.distance;
                self.add_hop_to_block_trace_data(
                    &mut block,
                    NetworkTraceHop {
                        endpoint: self.endpoint.clone(),
                        distance,
                        socket: NetworkTraceHopSocket::new(
                            self.get_com_interface_from_socket_uuid(
                                socket_uuid,
                            )
                            .borrow_mut()
                            .get_properties(),
                            socket_uuid.clone(),
                        ),
                        direction: NetworkTraceHopDirection::Outgoing,
                    },
                );
            }
            _ => {}
        }

        let socket = self.get_socket_by_uuid(socket_uuid);
        let mut socket_ref = socket.lock().unwrap();

        let is_broadcast = endpoints
            .iter()
            .any(|e| e == &Endpoint::ANY_ALL_INSTANCES || e == &Endpoint::ANY);

        if is_broadcast
            && let Some(direct_endpoint) = &socket_ref.direct_endpoint
            && (direct_endpoint == &self.endpoint
                || direct_endpoint == &Endpoint::LOCAL)
        {
            return;
        }

        match &block.to_bytes() {
            Ok(bytes) => {
                info!(
                    "Sending block to socket {}: {}",
                    socket_uuid,
                    endpoints.iter().map(|e| e.to_string()).join(", ")
                );

                // TODO: resend block if socket failed to send
                socket_ref.queue_outgoing_block(bytes);
            }
            Err(err) => {
                error!("Failed to convert block to bytes: {err:?}");
            }
        }
    }

    /// Updates all interfaces to handle reconnections if the interface can be reconnected
    /// or remove the interface if it cannot be reconnected.
    fn update_interfaces(&self) {
        let mut to_remove = Vec::new();
        for (interface, _) in self.interfaces.borrow().values() {
            let uuid = interface.borrow().get_uuid().clone();
            let state = interface.borrow().get_state();

            // If the interface has been proactively destroyed, remove it from the hub
            // and clean up the sockets. This happens when the user calls the destroy
            // method on the interface and not the remove_interface on the ComHub.
            if state.is_destroyed() {
                info!("Destroying interface on the ComHub {uuid}");
                to_remove.push(uuid);
            } else if state.is_not_connected()
                && interface.borrow_mut().get_properties().shall_reconnect()
            {
                // If the interface is disconnected and the interface has
                // reconnection enabled, check if the interface should be reconnected
                let interface_rc = interface.clone();
                let mut interface = interface.borrow_mut();

                let already_connecting =
                    interface.get_state() == ComInterfaceState::Connecting;

                if !already_connecting {
                    let config = interface.get_properties_mut();

                    let reconnect_now = match &config.reconnection_config {
                        ReconnectionConfig::InstantReconnect => true,
                        ReconnectionConfig::ReconnectWithTimeout { timeout } => {
                            ReconnectionConfig::check_reconnect_timeout(
                                config.close_timestamp,
                                timeout,
                            )
                        }
                        ReconnectionConfig::ReconnectWithTimeoutAndAttempts {
                            timeout,
                            attempts,
                        } => {
                            let max_attempts = attempts;

                            // check if the attempts are not exceeded
                            let attempts = config.reconnect_attempts.unwrap_or(0);
                            let attempts = attempts + 1;
                            if attempts > *max_attempts {
                                to_remove.push(uuid.clone());
                                return;
                            }

                            config.reconnect_attempts = Some(attempts);

                            ReconnectionConfig::check_reconnect_timeout(
                                config.close_timestamp,
                                timeout,
                            )
                        }
                        ReconnectionConfig::NoReconnect => false,
                    };
                    if reconnect_now {
                        debug!("Reconnecting interface {uuid}");
                        interface.set_state(ComInterfaceState::Connecting);
                        spawn_with_panic_notify(async move {
                            let interface = interface_rc.clone();
                            let mut interface = interface.borrow_mut();

                            let config = interface.get_properties_mut();
                            config.close_timestamp = None;

                            let current_attempts =
                                config.reconnect_attempts.unwrap_or(0);
                            config.reconnect_attempts =
                                Some(current_attempts + 1);

                            let res = interface.handle_open().await;
                            if res {
                                interface
                                    .set_state(ComInterfaceState::Connected);
                                // config.reconnect_attempts = None;
                            } else {
                                interface
                                    .set_state(ComInterfaceState::NotConnected);
                            }
                        });
                    } else {
                        debug!("Not reconnecting interface {uuid}");
                    }
                }
            }
        }

        for uuid in to_remove {
            self.cleanup_interface(uuid);
        }
    }

    /// Updates all known sockets for all interfaces to update routing
    /// information, remove deleted sockets and add new sockets and endpoint relations
    fn update_sockets(&self) {
        let mut new_sockets = Vec::new();
        let mut deleted_sockets = Vec::new();
        let mut registered_sockets = Vec::new();

        for (interface, priority) in self.interfaces.borrow().values() {
            let socket_updates = interface.clone().borrow().get_sockets();
            let mut socket_updates = socket_updates.lock().unwrap();

            registered_sockets
                .extend(socket_updates.socket_registrations.drain(..));
            new_sockets.extend(
                socket_updates.new_sockets.drain(..).map(|s| (s, *priority)),
            );
            deleted_sockets.extend(socket_updates.deleted_sockets.drain(..));
        }

        for (socket, priority) in new_sockets {
            self.add_socket(socket.clone(), priority);
        }
        for socket_uuid in deleted_sockets {
            self.delete_socket(&socket_uuid);
        }
        for (socket_uuid, distance, endpoint) in registered_sockets {
            let socket = self.get_socket_by_uuid(&socket_uuid);
            self.register_socket_endpoint(socket, endpoint.clone(), distance)
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to register socket {socket_uuid} for endpoint {endpoint} {e:?}"
                    );
                });
        }
    }

    /// Collects incoming data slices from all sockets. The sockets will call their
    /// BlockCollector to collect the data into blocks.
    fn collect_incoming_data(&self) {
        // update sockets, collect incoming data into full blocks
        for (socket, _) in self.sockets.borrow().values() {
            let mut socket_ref = socket.lock().unwrap();
            socket_ref.collect_incoming_data();
        }
    }

    /// Collects all blocks from the receive queues of all sockets and process them
    /// in the receive_block method.
    fn receive_incoming_blocks(&self) {
        let mut blocks = vec![];
        // iterate over all sockets
        for (socket, _) in self.sockets.borrow().values() {
            let mut socket_ref = socket.lock().unwrap();
            let uuid = socket_ref.uuid.clone();
            let block_queue = socket_ref.get_incoming_block_queue();
            blocks.push((uuid, block_queue.drain(..).collect::<Vec<_>>()));
        }

        for (uuid, blocks) in blocks {
            for block in blocks.iter() {
                self.receive_block(block, uuid.clone());
            }
        }
    }

    /// Sends all queued blocks from all interfaces.
    fn flush_outgoing_blocks(&self) {
        let interfaces = self.interfaces.borrow();
        for (interface, _) in interfaces.values() {
            com_interface::flush_outgoing_blocks(interface.clone());
        }
    }
}
