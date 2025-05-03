use crate::global::protocol_structures::block_header::BlockType;
use crate::global::protocol_structures::routing_header::{
    ReceiverEndpoints, SignatureType,
};
use crate::runtime::global_context::get_global_context;
use crate::stdlib::{cell::RefCell, rc::Rc};
use crate::task::spawn_local;
use futures_util::future::join_all;
use itertools::Itertools;
use log::{debug, error, info, warn};
use std::any::Any;
use std::cell::{Ref, RefMut};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(feature = "tokio_runtime")]
use tokio::task::yield_now;
// FIXME no-std

use super::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceState,
};
use super::com_interfaces::{
    com_interface::ComInterface, com_interface_socket::ComInterfaceSocket,
};
use crate::datex_values::{Endpoint, EndpointInstance};
use crate::global::dxb_block::DXBBlock;
use crate::network::block_handler::{BlockHandler, ResponseBlocks};
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
    pub interface_factories: HashMap<String, ComInterfaceFactoryFn>,

    /// a list of all available interfaces, keyed by their UUID
    pub interfaces: HashMap<ComInterfaceUUID, Rc<RefCell<dyn ComInterface>>>,

    /// a list of all available sockets, keyed by their UUID
    /// contains the socket itself and a list of endpoints currently associated with it
    pub sockets: HashMap<
        ComInterfaceSocketUUID,
        (Arc<Mutex<ComInterfaceSocket>>, HashSet<Endpoint>),
    >,

    /// a list of all available sockets for each endpoint, with additional
    /// DynamicEndpointProperties metadata
    pub endpoint_sockets: HashMap<
        Endpoint,
        Vec<(ComInterfaceSocketUUID, DynamicEndpointProperties)>,
    >,

    pub block_handler: Rc<RefCell<BlockHandler>>,

    /// the default socket for the hub to send outgoing block to
    /// if no socket is available for a receiver endpoint
    pub default_socket_uuid: Option<ComInterfaceSocketUUID>,

    /// the default interface for the hub to send outgoing block to
    /// if no interface is available for a receiver endpoint
    pub default_interface_uuid: Option<ComInterfaceUUID>,
}

#[derive(Debug, Clone, Default)]
struct EndpointIterateOptions<'a> {
    pub only_direct: bool,
    pub exact_instance: bool,
    pub exclude_socket: Option<&'a ComInterfaceSocketUUID>,
}

impl Default for ComHub {
    fn default() -> Self {
        ComHub {
            endpoint: Endpoint::default(),
            interface_factories: HashMap::new(),
            interfaces: HashMap::new(),
            endpoint_sockets: HashMap::new(),
            block_handler: Rc::new(RefCell::new(BlockHandler::new())),
            sockets: HashMap::new(),
            default_interface_uuid: None,
            default_socket_uuid: None,
        }
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

    pub async fn init(&mut self) -> Result<(), ComHubError> {
        // add default local loopback interface
        let local_interface = LocalLoopbackInterface::new();
        self.open_and_add_interface(Rc::new(RefCell::new(local_interface)))
            .await
    }

    /// Register a new interface factory for a specific interface implementation.
    /// This allows the ComHub to create new instances of the interface on demand.
    pub fn register_interface_factory(
        &mut self,
        interface_type: String,
        factory: ComInterfaceFactoryFn,
    ) {
        self.interface_factories.insert(interface_type, factory);
    }

    /// Create a new interface instance using the registered factory
    /// for the specified interface type if it exists.
    /// The interface is opened and added to the ComHub.
    pub async fn create_interface(
        &mut self,
        interface_type: &str,
        setup_data: Box<dyn Any>,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComHubError> {
        if let Some(factory) = self.interface_factories.get(interface_type) {
            let interface =
                factory(setup_data).map_err(ComHubError::InterfaceError)?;
            self.open_and_add_interface(interface.clone())
                .await
                .map(|_| interface)
        } else {
            Err(ComHubError::InterfaceTypeDoesNotExist)
        }
    }

    pub fn set_default_interface(
        &mut self,
        interface_uuid: ComInterfaceUUID,
    ) -> Result<(), ComHubError> {
        if self.interfaces.contains_key(&interface_uuid) {
            self.default_interface_uuid = Some(interface_uuid.clone());
            let socket_list = &self
                .get_com_interface_by_uuid(&interface_uuid)
                .borrow()
                .get_sockets();

            if let Some(socket) =
                socket_list.lock().unwrap().sockets.values().next()
            {
                self.default_socket_uuid =
                    Some(socket.lock().unwrap().uuid.clone());
            } else {
                debug!("No sockets found for interface {interface_uuid}");
            }

            Ok(())
        } else {
            Err(ComHubError::InterfaceDoesNotExist)
        }
    }

    pub fn get_default_interface(&self) -> Option<ComInterfaceUUID> {
        self.default_interface_uuid.clone()
    }

    pub fn get_interface_by_uuid_mut<T: ComInterface + 'static>(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Option<RefMut<T>> {
        let iface = self.interfaces.get(interface_uuid)?;
        let borrowed = iface.borrow_mut();
        RefMut::filter_map(borrowed, |b| b.as_any_mut().downcast_mut::<T>())
            .ok()
    }

    pub fn has_interface(&self, interface_uuid: &ComInterfaceUUID) -> bool {
        self.interfaces.contains_key(interface_uuid)
    }

    pub fn get_interface_by_uuid<T: ComInterface + 'static>(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Option<Ref<T>> {
        let iface = self.interfaces.get(interface_uuid)?;
        let borrowed = iface.borrow();
        Ref::filter_map(borrowed, |b| b.as_any().downcast_ref::<T>()).ok()
    }

    pub fn get_interface_ref_by_uuid(
        &self,
        uuid: &ComInterfaceUUID,
    ) -> Option<Rc<RefCell<dyn ComInterface>>> {
        self.interfaces.get(uuid).cloned()
    }

    pub async fn add_default_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> Result<(), ComHubError> {
        self.open_and_add_interface(interface.clone()).await?;
        let uuid = interface.borrow().get_uuid().clone();
        self.set_default_interface(uuid)?;
        Ok(())
    }

    pub async fn open_and_add_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> Result<(), ComHubError> {
        if interface.borrow().get_state() != ComInterfaceState::Connected {
            // If interface is not connected, open it
            // and wait for it to be connected
            interface.borrow_mut().handle_open().await;
        }
        self.add_interface(interface.clone())
    }

    pub fn add_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> Result<(), ComHubError> {
        let uuid = interface.borrow().get_uuid().clone();
        if self.interfaces.contains_key(&uuid) {
            return Err(ComHubError::InterfaceAlreadyExists);
        }
        self.interfaces.insert(uuid, interface);
        Ok(())
    }

    /// User can proactively remove an interface from the hub.
    /// This will destroy the interface and it's sockets (perform deep cleanup)
    pub async fn remove_interface(
        &mut self,
        interface_uuid: ComInterfaceUUID,
    ) -> Result<(), ComHubError> {
        info!("Removing interface {interface_uuid}");
        let interface: &Rc<RefCell<dyn ComInterface>> = self
            .interfaces
            .get_mut(&interface_uuid.clone())
            .ok_or(ComHubError::InterfaceDoesNotExist)?;
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
        &mut self,
        interface_uuid: ComInterfaceUUID,
    ) -> Option<Rc<RefCell<dyn ComInterface>>> {
        let interface = self.interfaces.remove(&interface_uuid).or(None)?;

        if self.default_interface_uuid == Some(interface_uuid.clone()) {
            self.default_interface_uuid = None;
            warn!(
                "Default interface {interface_uuid} removed. No default interface set."
            );
        }
        Some(interface)
    }

    pub(crate) fn receive_block(
        &mut self,
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
                        self.handle_trace_block(block, socket_uuid);
                        return;
                    }
                    BlockType::TraceBack => {
                        self.handle_trace_back_block(block, socket_uuid);
                        return;
                    }
                    _ => {
                        self.block_handler
                            .borrow()
                            .handle_incoming_block(block.clone());
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
                        BlockType::Trace => {
                            self.redirect_trace_block(
                                block,
                                socket_uuid.clone(),
                            );
                        }
                        BlockType::TraceBack => {
                            self.redirect_trace_block(
                                block,
                                socket_uuid.clone(),
                            );
                        }
                        _ => {
                            self.relay_block(
                                block.clone(),
                                remaining_receivers,
                                socket_uuid.clone(),
                            );
                        }
                    }
                }
            }
        }

        // assign endpoint to socket if none is assigned
        self.register_socket_endpoint_from_incoming_block(socket_uuid, block);
    }

    /// returns a list of all receivers from a given ReceiverEndpoints
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
        &mut self,
        socket_uuid: ComInterfaceSocketUUID,
        block: &DXBBlock,
    ) {
        let socket = self.get_socket_by_uuid(&socket_uuid);
        let mut socket_ref = socket.lock().unwrap();

        let distance = block.routing_header.distance;
        let sender = block.routing_header.sender.clone();

        // set as direct endpoint if distance = 0
        if socket_ref.direct_endpoint.is_none() && distance == 0 {
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

    /// Prepare a block and relay it to the given receivers.
    /// The routing distance is incremented by 1.
    fn relay_block(
        &self,
        block: DXBBlock,
        receivers: &[Endpoint],
        original_socket: ComInterfaceSocketUUID,
    ) {
        let mut block = block.clone();
        block.set_receivers(receivers);
        // increment distance for next hop
        block.routing_header.distance += 1;
        self.send_block(block, Some(&original_socket));
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
        &mut self,
        socket: Arc<Mutex<ComInterfaceSocket>>,
        endpoint: Endpoint,
        distance: u8,
    ) -> Result<(), SocketEndpointRegistrationError> {
        let socket_ref = socket.lock().unwrap();

        // if the registered endpoint is the same as the socket endpoint,
        // this is a direct socket to the endpoint
        let is_direct = socket_ref.direct_endpoint == Some(endpoint.clone());

        // cannot register endpoint if socket is not connected
        if !socket_ref.state.is_open() {
            return Err(SocketEndpointRegistrationError::SocketDisconnected);
        }

        // check if the socket is already registered for the endpoint
        if let Some(entries) = self.endpoint_sockets.get(&endpoint)
            && entries
                .iter()
                .any(|(socket_uuid, _)| socket_uuid == &socket_ref.uuid)
        {
            return Err(SocketEndpointRegistrationError::SocketEndpointAlreadyRegistered);
        }

        // add endpoint to socket endpoint list
        self.add_socket_endpoint(&socket_ref.uuid, endpoint.clone());

        // add socket to endpoint socket list
        self.add_endpoint_socket(
            &endpoint,
            socket_ref.uuid.clone(),
            distance,
            is_direct,
            socket_ref.channel_factor,
            socket_ref.direction.clone(),
        );

        // resort sockets for endpoint
        self.sort_sockets(&endpoint);

        Ok(())
    }

    fn add_endpoint_socket(
        &mut self,
        endpoint: &Endpoint,
        socket_uuid: ComInterfaceSocketUUID,
        distance: u8,
        is_direct: bool,
        channel_factor: u32,
        direction: InterfaceDirection,
    ) {
        if !self.endpoint_sockets.contains_key(endpoint) {
            self.endpoint_sockets.insert(endpoint.clone(), Vec::new());
        }

        let endpoint_sockets = self.endpoint_sockets.get_mut(endpoint).unwrap();
        endpoint_sockets.push((
            socket_uuid,
            DynamicEndpointProperties {
                known_since: 1, // FIXME
                distance,
                is_direct,
                channel_factor,
                direction,
            },
        ));
    }

    fn add_socket(&mut self, socket: Arc<Mutex<ComInterfaceSocket>>) {
        let socket_ref = socket.lock().unwrap();
        let socket_uuid = socket_ref.uuid.clone();
        if self.sockets.contains_key(&socket_ref.uuid) {
            panic!("Socket {} already exists in ComHub", socket_ref.uuid);
        }

        self.sockets
            .insert(socket_ref.uuid.clone(), (socket.clone(), HashSet::new()));
        // set as default socket if interface is registered as default interface
        if self.default_socket_uuid.is_none()
            && match self.default_interface_uuid {
                Some(ref default_interface) => {
                    socket_ref.interface_uuid == *default_interface
                }
                None => false,
            }
        {
            debug!(
                "Setting default socket for interface {}: {}",
                socket_ref.interface_uuid, socket_ref.uuid
            );
            self.default_socket_uuid = Some(socket_ref.uuid.clone());
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

    fn delete_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        self.sockets
            .remove(socket_uuid)
            .or_else(|| panic!("Socket {socket_uuid} not found in ComHub"));

        // remove socket from endpoint socket list
        // remove endpoint key from endpoint_sockets if not sockets present
        self.endpoint_sockets.retain(|_, sockets| {
            sockets.retain(|(uuid, _)| uuid != socket_uuid);
            !sockets.is_empty()
        });

        // remove socket if it is the default socket
        if self.default_socket_uuid == Some(socket_uuid.clone()) {
            self.default_socket_uuid = None;
        }
    }

    fn add_socket_endpoint(
        &mut self,
        socket_uuid: &ComInterfaceSocketUUID,
        endpoint: Endpoint,
    ) {
        assert!(
            self.sockets.contains_key(socket_uuid),
            "Socket not found in ComHub"
        );
        // add endpoint to socket endpoint list
        self.sockets
            .get_mut(socket_uuid)
            .unwrap()
            .1
            .insert(endpoint.clone());
    }

    /// Sort the sockets for an endpoint:
    /// - socket with send capability first
    /// - then direct sockets
    /// - then sort by channel channel_factor (latency, bandwidth)
    /// - then sort by socket connect_timestamp
    fn sort_sockets(&mut self, endpoint: &Endpoint) {
        let sockets = self.endpoint_sockets.get_mut(endpoint).unwrap();

        sockets.sort_by(|(_, a), (_, b)| {
            // sort by channel_factor
            b.is_direct
                .cmp(&a.is_direct)
                .then_with(|| b.channel_factor.cmp(&a.channel_factor))
                .then_with(|| b.distance.cmp(&a.distance))
                .then_with(|| b.known_since.cmp(&a.known_since))
        });
    }

    pub(crate) fn get_socket_by_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Arc<Mutex<ComInterfaceSocket>> {
        self.sockets
            .get(socket_uuid)
            .map(|socket| socket.0.clone())
            .unwrap_or_else(|| {
                panic!("Socket for uuid {socket_uuid} not found")
            })
    }

    pub(crate) fn get_com_interface_by_uuid(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Rc<RefCell<dyn ComInterface>> {
        self.interfaces
            .get(interface_uuid)
            .unwrap_or_else(|| {
                panic!("Interface for uuid {interface_uuid} not found")
            })
            .clone()
    }

    pub(crate) fn get_com_interface_from_socket_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Rc<RefCell<dyn ComInterface>> {
        let socket = self.get_socket_by_uuid(socket_uuid);
        let socket = socket.lock().unwrap();
        self.get_com_interface_by_uuid(&socket.interface_uuid)
    }

    fn get_socket_interface_properties(
        interfaces: &HashMap<ComInterfaceUUID, Rc<RefCell<dyn ComInterface>>>,
        interface_uuid: &ComInterfaceUUID,
    ) -> InterfaceProperties {
        interfaces
            .get(interface_uuid)
            .unwrap()
            .borrow()
            .init_properties()
    }

    fn iterate_endpoint_sockets<'a>(
        &'a self,
        endpoint: &'a Endpoint,
        options: EndpointIterateOptions<'a>,
    ) -> impl Iterator<Item = ComInterfaceSocketUUID> + 'a {
        let endpoint_sockets = self.endpoint_sockets.get(endpoint);
        std::iter::from_coroutine(
            #[coroutine]
            move || {
                if endpoint_sockets.is_none() {
                    return;
                }
                for (socket_uuid, _) in endpoint_sockets.unwrap() {
                    {
                        let socket = self.get_socket_by_uuid(socket_uuid);
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
                        if let Some(exclude_socket) = &options.exclude_socket
                            && &socket.uuid == *exclude_socket
                        {
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
                            break;
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
        exclude_socket: Option<&ComInterfaceSocketUUID>,
    ) -> Option<ComInterfaceSocketUUID> {
        match endpoint.instance {
            // find socket for any endpoint instance
            EndpointInstance::Any => {
                let options = EndpointIterateOptions {
                    only_direct: false,
                    exact_instance: false,
                    exclude_socket,
                };
                if let Some(socket) =
                    self.iterate_endpoint_sockets(endpoint, options).next()
                {
                    // TODO
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
                    exclude_socket,
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
        exclude_socket: Option<&ComInterfaceSocketUUID>,
    ) -> Option<ComInterfaceSocketUUID> {
        // find best known socket for endpoint
        let matching_socket =
            self.find_known_endpoint_socket(endpoint, exclude_socket);

        // if a matching socket is found, return it
        if matching_socket.is_some() {
            matching_socket
        }
        // otherwise, return the default socket if it exists and is not excluded
        else if self.default_socket_uuid.is_some()
            && (exclude_socket.is_none()
                || &self.default_socket_uuid.clone().unwrap()
                    != exclude_socket.unwrap())
        {
            Some(self.default_socket_uuid.clone().unwrap())
        } else {
            None
        }
    }

    /// returns all receivers to which the block has to be sent, grouped by the
    /// outbound socket uuids
    fn get_outbound_receiver_groups(
        &self,
        block: &DXBBlock,
        exclude_socket: Option<&ComInterfaceSocketUUID>,
    ) -> Option<Vec<(Option<ComInterfaceSocketUUID>, Vec<Endpoint>)>> {
        if let Some(receivers) = block.receivers() {
            if !receivers.is_empty() {
                let endpoint_sockets = receivers
                    .iter()
                    .map(|e| {
                        let socket =
                            self.find_best_endpoint_socket(e, exclude_socket);
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

    /// Run the update loop for the ComHub.
    /// This method will continuously handle incoming data, send out
    /// queued blocks and update the sockets.
    pub fn start_update_loop(self_rc: Rc<RefCell<Self>>) {
        spawn_local(async move {
            loop {
                ComHub::update(self_rc.clone()).await;
                #[cfg(feature = "tokio_runtime")]
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
    }

    /// Update all sockets and interfaces,
    /// collecting incoming data and sending out queued blocks.
    pub async fn update(self_rc: Rc<RefCell<Self>>) {
        // 1. self_rc.lock
        {
            info!("running ComHub update loop...");
            let mut self_ref = self_rc.borrow_mut();

            // update all interfaces
            self_ref.update_interfaces().await;

            // update own socket lists for routing
            self_ref.update_sockets();

            // update sockets block collectors
            self_ref.collect_incoming_data();

            // receive blocks from all sockets
            self_ref.receive_incoming_blocks();
            info!("done...");
        }

        // send all queued blocks from all interfaces
        ComHub::flush_outgoing_blocks(self_rc.clone()).await;
    }

    /// Prepare a block for sending out by updating the creation timestamp,
    /// sender and add signature and encryption if needed.
    fn prepare_own_block(&self, mut block: DXBBlock) -> DXBBlock {
        // TODO signature & encryption
        let now = get_global_context().clone().time.lock().unwrap().now();
        block.routing_header.sender = self.endpoint.clone();
        block
            .block_header
            .flags_and_timestamp
            .set_creation_timestamp(now);
        block
    }

    /// Public method to send an outgoing block from this endpoint. Called by the runtime.
    pub fn send_own_block(&self, mut block: DXBBlock) {
        block = self.prepare_own_block(block);
        self.send_block(block, None);
    }

    /// Send a block and wait for a response block.
    pub async fn send_own_block_await_response(
        self_rc: Rc<RefCell<Self>>,
        block: DXBBlock,
    ) -> Result<ResponseBlocks, ComHubError> {
        let scope_id = block.block_header.scope_id;
        let block_index = block.block_header.block_index;
        {
            let self_ref = self_rc.borrow();
            self_ref.send_own_block(block);
        }
        // yield
        #[cfg(feature = "tokio_runtime")]
        yield_now().await;
        log::info!("awaited blok");

        let block_handler = self_rc.borrow().block_handler.clone();
        let res = block_handler
            .borrow()
            .wait_for_incoming_response_block(scope_id, block_index)
            .await
            .ok_or(ComHubError::NoResponse);

        res
    }

    /// Send a block to all endpoints specified in the block header.
    /// The routing algorithm decides which sockets are used to send the block, based on the endpoint.
    /// A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
    /// The original_socket parameter is used to prevent sending the block back to the sender.
    /// When this method is called, the block is queued in the send queue.
    pub fn send_block(
        &self,
        block: DXBBlock,
        original_socket: Option<&ComInterfaceSocketUUID>,
    ) {
        let outbound_receiver_groups =
            self.get_outbound_receiver_groups(&block, original_socket);

        if outbound_receiver_groups.is_none() {
            error!("No outbound receiver groups found for block");
            return;
        }

        let outbound_receiver_groups = outbound_receiver_groups.unwrap();

        for (receiver_socket, endpoints) in outbound_receiver_groups {
            if let Some(socket) = receiver_socket {
                self.send_block_addressed(block.clone(), &socket, &endpoints);
            } else {
                error!("Cannot send block, no receiver sockets found for endpoints {:?}", endpoints.iter().map(|e| e.to_string()).collect::<Vec<_>>());
            }
        }
    }

    /// Send a block via a socket to a list of endpoints.
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
                self.add_hop_to_block_trace_data(
                    &mut block,
                    NetworkTraceHop {
                        endpoint: self.endpoint.clone(),
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

    /// Update all interfaces to handle reconnections if the interface can be reconnected
    /// or remove the interface if it cannot be reconnected.
    async fn update_interfaces(&mut self) {
        let local_set = tokio::task::LocalSet::new();

        let mut to_remove = Vec::new();
        for interface in self.interfaces.values() {
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

                        // check if the attemps are not exceeded
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
                drop(interface);
                if reconnect_now {
                    debug!("Reconnecting interface {uuid}");
                    local_set.spawn_local(async move {
                        // FIXME
                        let interface = interface_rc.clone();
                        let mut interface = interface.borrow_mut();
                        interface.set_state(ComInterfaceState::Connecting);

                        let config = interface.get_properties_mut();
                        config.close_timestamp = None;

                        let current_attempts =
                            config.reconnect_attempts.unwrap_or(0);
                        config.reconnect_attempts = Some(current_attempts + 1);

                        let res = interface.handle_open().await;
                        if res {
                            interface.set_state(ComInterfaceState::Connected);
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

        for uuid in to_remove {
            self.cleanup_interface(uuid);
        }
        local_set.await;
    }

    /// Update all known sockets for all interfaces to update routing
    /// information, remove deleted sockets and add new sockets and endpoint relations
    fn update_sockets(&mut self) {
        let mut new_sockets = Vec::new();
        let mut deleted_sockets = Vec::new();
        let mut registered_sockets = Vec::new();

        for interface in self.interfaces.values() {
            let socket_updates = interface.clone().borrow().get_sockets();
            let mut socket_updates = socket_updates.lock().unwrap();

            registered_sockets
                .extend(socket_updates.socket_registrations.drain(..));
            new_sockets.extend(socket_updates.new_sockets.drain(..));
            deleted_sockets.extend(socket_updates.deleted_sockets.drain(..));
        }

        for socket in new_sockets {
            self.add_socket(socket.clone());
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

    /// Collect incoming data slices from all sockets. The sockets will call their
    /// BlockCollector to collect the data into blocks.
    fn collect_incoming_data(&self) {
        // update sockets, collect incoming data into full blocks
        info!("Collecting incoming data from all sockets");
        for (socket, _) in self.sockets.values() {
            let mut socket_ref = socket.lock().unwrap();
            socket_ref.collect_incoming_data();
        }
    }

    /// Collect all blocks from the receive queues of all sockets and process them
    /// in the receive_block method.
    fn receive_incoming_blocks(&mut self) {
        let mut blocks = vec![];
        // iterate over all sockets
        for (socket, _) in self.sockets.values() {
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

    /// Send all queued blocks from all interfaces.
    async fn flush_outgoing_blocks(self_rc: Rc<RefCell<Self>>) {
        // TODO: more efficient way than cloning into vec? self_rc lock must not exist after this
        let interfaces = {
            let guard = self_rc.borrow();
            guard.interfaces.values().cloned().collect::<Vec<_>>()
        };
        join_all(interfaces.iter().map(|interface| {
            Box::pin(async move {
                let mut interface = interface.borrow_mut();
                interface.flush_outgoing_blocks().await
            })
        }))
        .await;
    }
}
