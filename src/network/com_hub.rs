use crate::stdlib::collections::VecDeque;
use crate::stdlib::{cell::RefCell, rc::Rc};
use futures_util::future::join_all;
use itertools::Itertools;
use log::{debug, error, info};
use std::cell::{Ref, RefMut};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use tokio::task::spawn_local;
// FIXME no-std

use super::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceState,
};
use super::com_interfaces::{
    com_interface::ComInterface, com_interface_socket::ComInterfaceSocket,
};
use crate::datex_values::{Endpoint, EndpointInstance};
use crate::global::dxb_block::DXBBlock;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;

#[derive(Debug, Clone)]
pub struct DynamicEndpointProperties {
    pub known_since: u64,
    pub distance: u32,
    pub is_direct: bool,
    pub channel_factor: u32,
    pub direction: InterfaceDirection,
}

pub struct ComHub {
    pub endpoint: Endpoint,

    pub interfaces: HashMap<ComInterfaceUUID, Rc<RefCell<dyn ComInterface>>>,
    /// a list of all available sockets, keyed by their UUID
    /// contains the socket itself and a list of endpoints currently associated with it
    // TODO: keep socket mapping up to date
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
    pub incoming_blocks: Rc<RefCell<VecDeque<Rc<DXBBlock>>>>,
    pub default_socket_uuid: Option<ComInterfaceSocketUUID>,
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
            interfaces: HashMap::new(),
            endpoint_sockets: HashMap::new(),
            incoming_blocks: Rc::new(RefCell::new(VecDeque::new())),
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
}

#[derive(Debug)]
pub enum SocketEndpointRegistrationError {
    SocketDisconnected,
    SocketUninitialized,
}

impl ComHub {
    pub fn new(endpoint: Endpoint) -> Arc<Mutex<ComHub>> {
        Arc::new(Mutex::new(ComHub {
            endpoint,
            ..ComHub::default()
        }))
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
                debug!("No sockets found for interface {}", interface_uuid);
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
    pub fn get_interface_by_uuid<T: ComInterface + 'static>(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Option<Ref<T>> {
        let iface = self.interfaces.get(interface_uuid)?;
        let borrowed = iface.borrow();
        Ref::filter_map(borrowed, |b| b.as_any().downcast_ref::<T>()).ok()
    }

    pub async fn add_default_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> Result<(), ComHubError> {
        self.add_interface(interface.clone()).await?;
        let uuid = interface.borrow().get_uuid().clone();
        self.set_default_interface(uuid)?;
        Ok(())
    }

    pub async fn add_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> Result<(), ComHubError> {
        if interface.borrow().get_state() != ComInterfaceState::Connected {
            return Err(ComHubError::InterfaceNotConnected);
        }
        let uuid = interface.borrow().get_uuid().clone();
        if self.interfaces.contains_key(&uuid) {
            return Err(ComHubError::InterfaceAlreadyExists);
        }
        self.interfaces.insert(uuid, interface);
        Ok(())
    }

    /// User can remove an interface from the hub.
    /// This will destroy the interface and it's sockets (perform deep cleanup)
    pub async fn remove_interface(
        &mut self,
        interface_uuid: ComInterfaceUUID,
    ) -> Result<(), ComHubError> {
        info!("Removing interface {}", interface_uuid);
        let interface: &Rc<RefCell<dyn ComInterface>> = self
            .interfaces
            .get_mut(&interface_uuid.clone())
            .ok_or(ComHubError::InterfaceDoesNotExist)?;
        {
            // Async close the interface (stop tasks, server, cleanup internal data)
            let interface = interface.clone();
            let mut interface = interface.borrow_mut();
            if !interface.close().await {
                return Err(ComHubError::InterfaceCloseFailed);
            }
        }
        {
            let interface = interface.clone();
            let mut interface = interface.borrow_mut();

            // Remove the sockets from the socket list
            // to notify ComHub routing logic
            interface.destroy_sockets();
        }

        self.cleanup_interface(interface_uuid)
            .ok_or(ComHubError::InterfaceDoesNotExist)?;

        Ok(())
    }

    fn cleanup_interface(
        &mut self,
        interface_uuid: ComInterfaceUUID,
    ) -> Option<Rc<RefCell<dyn ComInterface>>> {
        let interface = self.interfaces.remove(&interface_uuid).or(None)?;

        if self.default_interface_uuid == Some(interface_uuid) {
            self.default_interface_uuid = None;
        }
        Some(interface)
    }

    pub(crate) fn receive_block(
        &self,
        block: &DXBBlock,
        socket_uuid: &ComInterfaceSocketUUID,
    ) {
        info!("Received block addressed to {:?}", block.receivers());

        if let Some(receivers) = &block.routing_header.receivers.endpoints {
            let is_for_own = receivers.endpoints.contains(&self.endpoint);
            // check if the block is for own endpoint
            if is_for_own {
                info!("Block is for this endpoint");
                let mut incoming_blocks = self.incoming_blocks.borrow_mut();
                incoming_blocks.push_back(Rc::new(block.clone()));
            }

            // get all receivers that the block must be relayed to
            let remaining_receivers = if is_for_own {
                &receivers
                    .endpoints
                    .iter()
                    .filter(|e| e != &&self.endpoint)
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                &receivers.endpoints
            };

            if !remaining_receivers.is_empty() {
                let block = &mut block.clone();
                block.set_receivers(remaining_receivers);
                self.send_block(block, Some(socket_uuid));
            }
        }

        // own incoming blocks
    }

    // TODO this method is currently not beeing invoked
    // We have to finalize the get_com_interface_sockets logic and empty the add registered endpoint socket queue
    // to call the register_socket_endpoint on the comhub during updates to be able to sort the endpoint sockets
    // for priority

    /// registers a new endpoint that is reachable over the socket
    /// if the socket is not already registered, it will be added to the socket list
    /// if the provided endpoint is not the same as the socket endpoint,
    /// it is registered as an indirect socket to the endpoint
    pub fn register_socket_endpoint(
        &mut self,
        socket: Arc<Mutex<ComInterfaceSocket>>,
        endpoint: Endpoint,
        distance: u32,
    ) -> Result<(), SocketEndpointRegistrationError> {
        let socket_ref = socket.lock().unwrap();
        // if the registered endpoint is the same as the socket endpoint,
        // this is a direct socket to the endpoint
        let is_direct = socket_ref.direct_endpoint == Some(endpoint.clone());

        // cannot register endpoint if socket is not connected
        if !socket_ref.is_connected {
            return Err(SocketEndpointRegistrationError::SocketDisconnected);
        }

        // cannot register endpoint if socket is not initialized (no endpoint assigned)
        if socket_ref.direct_endpoint.is_none() {
            return Err(SocketEndpointRegistrationError::SocketUninitialized);
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
        distance: u32,
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
    }

    fn delete_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        self.sockets
            .remove(socket_uuid)
            .or_else(|| panic!("Socket {} not found in ComHub", socket_uuid));
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
                panic!("Socket for uuid {} not found", socket_uuid)
            })
    }

    pub(crate) fn get_com_interface_by_uuid(
        &self,
        interface_uuid: &ComInterfaceUUID,
    ) -> Rc<RefCell<dyn ComInterface>> {
        self.interfaces
            .get(interface_uuid)
            .unwrap_or_else(|| {
                panic!("Interface for uuid {} not found", interface_uuid)
            })
            .clone()
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
                        info!("Socket UUID 123: {:?}", socket_uuid);
                        let socket = self.get_socket_by_uuid(socket_uuid);
                        let socket = socket.lock().unwrap();

                        // check if only_direct is set and the endpoint equals the direct endpoint of the socket
                        if options.only_direct
                            && socket.direct_endpoint.is_some()
                            && socket.direct_endpoint.as_ref().unwrap()
                                == endpoint
                        {
                            debug!(
                                "No direct socket found for endpoint {}. Skipping...",
                                endpoint
                            );
                            continue;
                        }

                        // check if the socket is excluded if exclude_socket is set
                        if let Some(exclude_socket) = &options.exclude_socket {
                            if &socket.uuid == *exclude_socket {
                                debug!(
                                    "Socket {} is excluded for endpoint {}. Skipping...",
                                    socket.uuid,
                                    endpoint
                                );
                                continue;
                            }
                        }

                        // only yield outgoing sockets
                        // if a non-outgoing socket is found, all following sockets
                        // will also be non-outgoing
                        if !socket.can_send() {
                            break;
                        }
                    }
                    debug!(
                        "Found matching socket {} for endpoint {}",
                        socket_uuid, endpoint
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
                for socket in self.iterate_endpoint_sockets(endpoint, options) {
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
                for socket in self.iterate_endpoint_sockets(endpoint, options) {
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
    /// outbound socket
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
                self_rc.borrow_mut().update().await;
            }
        });
    }

    /// Update all sockets and interfaces,
    /// collecting incoming data and sending out queued blocks.
    pub async fn update(&mut self) {
        info!("running ComHub update loop...");
        self.update_sockets();

        // update sockets block collectors
        self.collect_incoming_data();

        // receive blocks from all sockets
        self.receive_incoming_blocks();

        // send all queued blocks from all interfaces
        self.flush_outgoing_blocks().await;
    }

    /// Send a block to all endpoints specified in the block header.
    /// The routing algorithm decides which sockets are used to send the block, based on the endpoint.
    /// A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
    /// The original_socket parameter is used to prevent sending the block back to the sender.
    /// When this method is called, the block is queued in the send queue.
    pub fn send_block(
        &self,
        block: &DXBBlock,
        original_socket: Option<&ComInterfaceSocketUUID>,
    ) {
        let outbound_receiver_groups =
            self.get_outbound_receiver_groups(block, original_socket);

        if outbound_receiver_groups.is_none() {
            error!("No outbound receiver groups found for block");
            return;
        }

        let outbound_receiver_groups = outbound_receiver_groups.unwrap();

        for (receiver_socket, endpoints) in outbound_receiver_groups {
            if let Some(socket) = receiver_socket {
                self.send_block_addressed(block, &socket, &endpoints);
            } else {
                error!("Cannot send block, no receiver sockets found for endpoints {:?}", endpoints.iter().map(|e| e.to_string()).collect::<Vec<_>>());
            }
        }
    }

    /// Send a block via a socket to a list of endpoints.
    /// Before the block is sent, it is modified to include the list of endpoints as receivers.
    fn send_block_addressed(
        &self,
        block: &DXBBlock,
        socket_uuid: &ComInterfaceSocketUUID,
        endpoints: &[Endpoint],
    ) {
        let mut addressed_block = block.clone();
        addressed_block.set_receivers(endpoints);

        let socket = self.get_socket_by_uuid(socket_uuid);
        let mut socket_ref = socket.lock().unwrap();
        match &addressed_block.to_bytes() {
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
                error!("Failed to convert block to bytes: {:?}", err);
            }
        }
    }

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
                        "Failed to register socket {} for endpoint {} {:?}",
                        socket_uuid, endpoint, e
                    );
                });
        }
    }

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
        // iterate over all sockets
        for (socket, _) in self.sockets.values() {
            let socket_ref = socket.lock().unwrap();
            let block_queue = socket_ref.get_incoming_block_queue();
            let uuid = &socket_ref.uuid;
            for block in block_queue {
                self.receive_block(block, uuid);
            }
        }
    }

    /// Send all queued blocks from all interfaces.
    async fn flush_outgoing_blocks(&mut self) {
        join_all(self.interfaces.values().map(|interface| {
            Box::pin(async move {
                let mut interface = interface.borrow_mut();
                interface.flush_outgoing_blocks().await
            })
        }))
        .await;
    }
}
