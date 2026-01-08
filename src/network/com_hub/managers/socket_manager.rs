use crate::stdlib::rc::Rc;
use crate::task::{spawn_local, spawn_with_panic_notify, UnboundedReceiver, UnboundedSender};
use itertools::Itertools;
use log::{debug, error, info};

use crate::collections::{HashMap, HashSet};
use crate::network::com_hub::{BlockSendEvent, ComHub, ComHubError, InterfacePriority, SocketEndpointRegistrationError};
use crate::network::com_interfaces::com_interface_old::{
    ComInterfaceOld, ComInterfaceSocketEvent,
};
use crate::stdlib::sync::{Arc, Mutex};

use crate::utils::time::Time;
use crate::values::core_values::endpoint::EndpointInstance;
use crate::{
    network::com_interfaces::{
        com_interface_properties::InterfaceDirection,
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
    },
    values::core_values::endpoint::Endpoint,
};
use crate::global::dxb_block::DXBBlock;
use crate::runtime::AsyncContext;

pub type SocketsByUUID = HashMap<
    ComInterfaceSocketUUID,
    (Arc<Mutex<ComInterfaceSocket>>, HashSet<Endpoint>),
>;

#[derive(Debug, Clone, Default)]
pub struct EndpointIterateOptions<'a> {
    pub only_direct: bool,
    pub exact_instance: bool,
    pub exclude_sockets: &'a [ComInterfaceSocketUUID],
}

#[derive(Debug, Clone)]
pub struct DynamicEndpointProperties {
    pub known_since: u64,
    pub distance: i8,
    pub is_direct: bool,
    pub channel_factor: u32,
    pub direction: InterfaceDirection,
}

pub struct SocketManager {
    /// a list of all available sockets, keyed by their UUID
    /// contains the socket itself and a list of endpoints currently associated with it
    pub sockets: SocketsByUUID,

    /// a blacklist of sockets that are not allowed to be used for a specific endpoint
    pub endpoint_sockets_blacklist:
        HashMap<Endpoint, HashSet<ComInterfaceSocketUUID>>,

    /// fallback sockets that are used if no direct endpoint reachable socket is available
    /// sorted by priority
    pub fallback_sockets:
        Vec<(ComInterfaceSocketUUID, u16, InterfaceDirection)>,

    /// a list of all available sockets for each endpoint, with additional
    /// DynamicEndpointProperties metadata
    pub endpoint_sockets: HashMap<
        Endpoint,
        Vec<(ComInterfaceSocketUUID, DynamicEndpointProperties)>,
    >,

    /// sender to send hello requests to newly added sockets
    block_event_sender: UnboundedSender<BlockSendEvent>,
}
impl SocketManager {
    pub fn new(
        block_event_sender: UnboundedSender<BlockSendEvent>,
    ) -> SocketManager {
        SocketManager {
            sockets: HashMap::new(),
            endpoint_sockets_blacklist: HashMap::new(),
            fallback_sockets: Vec::new(),
            endpoint_sockets: HashMap::new(),
            block_event_sender,
        }
    }
}

/// Manages all sockets registered in the ComHub
/// Handles socket registration, endpoint registration and socket selection for endpoints
/// Also manages fallback sockets for outgoing connections and other lifelcycle events
impl SocketManager {
    /// Add a socket to the blocklist for a specific endpoint
    pub fn add_to_endpoint_blocklist(
        &mut self,
        endpoint: Endpoint,
        socket_uuid: &ComInterfaceSocketUUID,
    ) {
        self.endpoint_sockets_blacklist
            .entry(endpoint)
            .or_default()
            .insert(socket_uuid.clone());
    }

    /// Registers a new endpoint that is reachable over the socket if the socket is not
    /// already registered, it will be added to the socket list.
    /// If the provided endpoint is not the same as the socket endpoint, it is registered
    /// as an indirect socket to the endpoint
    pub fn register_socket_endpoint(
        &mut self,
        socket: Arc<Mutex<ComInterfaceSocket>>,
        endpoint: Endpoint,
        distance: i8,
    ) -> Result<(), SocketEndpointRegistrationError> {
        log::info!(
            "Registering endpoint {} for socket {}",
            endpoint,
            socket.try_lock().unwrap().uuid
        );
        let socket_ref = socket.try_lock().unwrap();

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

    /// Adds a socket to the socket list for a specific endpoint,
    /// attaching metadata as DynamicEndpointProperties
    fn add_endpoint_socket(
        &mut self,
        endpoint: &Endpoint,
        socket_uuid: ComInterfaceSocketUUID,
        distance: i8,
        is_direct: bool,
        channel_factor: u32,
        direction: InterfaceDirection,
    ) {
        if !self.endpoint_sockets.contains_key(endpoint) {
            self.endpoint_sockets.insert(endpoint.clone(), Vec::new());
        }

        self.endpoint_sockets.get_mut(endpoint).unwrap().push((
            socket_uuid,
            DynamicEndpointProperties {
                known_since: Time::now(),
                distance,
                is_direct,
                channel_factor,
                direction,
            },
        ));
    }

    /// Adds an endpoint to the endpoint list of a specific socket
    fn add_socket_endpoint(
        &mut self,
        socket_uuid: &ComInterfaceSocketUUID,
        endpoint: Endpoint,
    ) {
        core::assert!(
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

    /// Sorts the sockets for an endpoint:
    /// - socket with send capability first
    /// - then direct sockets
    /// - then sort by channel channel_factor (latency, bandwidth)
    /// - then sort by socket connect_timestamp
    ///
    /// When the global debug flag `enable_deterministic_behavior` is set,
    /// Sockets are not sorted by their connect_timestamp to make sure that the order of
    /// received blocks has no effect on the routing priorities
    fn sort_sockets(&mut self, endpoint: &Endpoint) {
        let sockets = self.endpoint_sockets.get_mut(endpoint).unwrap();

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
                                use crate::runtime::global_context::get_global_context;
                                use core::cmp::Ordering;
                                if get_global_context().debug_flags.enable_deterministic_behavior {
                                    Ordering::Equal
                                }
                                else {
                                    b.known_since.cmp(&a.known_since)
                                }
                            }
                            else {
                                b.known_since.cmp(&a.known_since)
                            }
                        }
                    }
                )

        });
    }

    /// Returns the socket for a given UUID
    /// The socket must be registered in the ComHub,
    /// otherwise a panic will be triggered
    pub(crate) fn socket_by_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Arc<Mutex<ComInterfaceSocket>> {
        self.sockets
            .get(socket_uuid)
            .map(|socket| socket.0.clone())
            .unwrap_or_else(|| {
                core::panic!("Socket for uuid {socket_uuid} not found")
            })
    }

    pub fn has_socket(&self, socket_uuid: &ComInterfaceSocketUUID) -> bool {
        self.sockets.contains_key(socket_uuid)
    }

    /// Adds a socket to the SocketManager
    /// Panics if the socket already exists
    fn add_socket_to_list(&mut self, socket_uuid: ComInterfaceSocketUUID, socket: Arc<Mutex<ComInterfaceSocket>>) {
        if self.has_socket(&socket_uuid) {
            core::panic!(
                "Socket {} already exists in SocketManager",
                socket_uuid
            );
        }
        self.sockets.insert(socket_uuid, (socket, HashSet::new()));
    }

    /// Adds a socket to the socket list.
    /// If the priority is not set to `InterfacePriority::None`, the socket
    /// is also registered as a fallback socket for outgoing connections with the
    /// specified priority.
    fn handle_new_socket(
        &mut self,
        socket: Arc<Mutex<ComInterfaceSocket>>,
        priority: InterfacePriority,
    ) -> Result<(), ComHubError> {
        let socket_ref = socket.try_lock().unwrap();
        let can_send = socket_ref.can_send();
        let socket_uuid = socket_ref.uuid.clone();
        if self.has_socket(&socket_ref.uuid) {
            core::panic!("Socket {} already exists in ComHub", socket_ref.uuid);
        }

        // info!(
        //     "Adding socket {} to ComHub with priority {:?}",
        //     socket_ref.uuid, priority
        // );

        if !socket_ref.can_send() && priority != InterfacePriority::None {
            core::panic!(
                "Socket {} cannot be used for fallback routing, since it has no send capability",
                socket_ref.uuid
            );
        }
        let direction = socket_ref.direction.clone();
        drop(socket_ref);

        self.add_socket_to_list(socket_uuid.clone(), socket.clone());

        // add outgoing socket to fallback sockets list if they have a priority flag
        if can_send {
            match priority {
                InterfacePriority::None => {
                    // do nothing
                }
                InterfacePriority::Priority(priority) => {
                    // add socket to fallback sockets list
                    self.add_fallback_socket(&socket_uuid, priority, direction);
                }
            }

        }

        // notify com hub about new socket so that it can init the socket task and optionally send a
        // hello block

        self.block_event_sender
            .start_send(BlockSendEvent::NewSocket {
                socket_uuid,
            })
            .expect("Can not send hello request to socket");
        Ok(())
    }

    /// Registers a socket as a fallback socket for outgoing connections
    /// that can be used if no known route exists for an endpoint
    /// Note: only sockets that support sending data should be used as fallback sockets
    pub fn add_fallback_socket(
        &mut self,
        socket_uuid: &ComInterfaceSocketUUID,
        priority: u16,
        direction: InterfaceDirection,
    ) {
        // add to vec
        self.fallback_sockets
            .push((socket_uuid.clone(), priority, direction));
        // first sort by direction (InOut before Out - only In is not allowed)
        // second sort by priority
        self.fallback_sockets.sort_by_key(|(_, priority, direction)| {
            let dir_rank = match direction {
                InterfaceDirection::InOut => 0,
                InterfaceDirection::Out => 1,
                InterfaceDirection::In => {
                    core::panic!("Socket {socket_uuid} is not allowed to be used as fallback socket")
                }
            };
            (dir_rank, core::cmp::Reverse(*priority))
        });
    }

    /// Removes a socket from the socket list
    pub fn delete_socket(&mut self, socket_uuid: &ComInterfaceSocketUUID) {
        if self.has_socket(socket_uuid) {
            core::panic!("Socket {socket_uuid} not found in ComHub")
        };

        // remove socket from endpoint socket list
        // remove endpoint key from endpoint_sockets if not sockets present
        self.endpoint_sockets.retain(|_, sockets| {
            sockets.retain(|(uuid, _)| uuid != socket_uuid);
            !sockets.is_empty()
        });

        // remove socket if it is the default socket
        self.fallback_sockets
            .retain(|(uuid, _, _)| uuid != socket_uuid);
    }

    /// Returns an iterator over all sockets for a given endpoint
    /// The sockets are yielded in the order of their priority, starting with the
    /// highest priority socket (the best socket for sending data to the endpoint)
    pub fn iterate_endpoint_sockets<'a>(
        &'a self,
        endpoint: &'a Endpoint,
        options: EndpointIterateOptions<'a>,
    ) -> impl Iterator<Item = ComInterfaceSocketUUID> + 'a {
        core::iter::from_coroutine(
            #[coroutine]
            move || {
                // TODO #183: can we optimize this to avoid cloning the endpoint_sockets vector?
                let endpoint_sockets =
                    self.endpoint_sockets.get(endpoint).cloned();
                if endpoint_sockets.is_none() {
                    return;
                }
                for (socket_uuid, _) in endpoint_sockets.unwrap() {
                    {
                        let socket = self.socket_by_uuid(&socket_uuid);
                        let socket = socket.try_lock().unwrap();

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
                                socket.uuid, endpoint
                            );
                            continue;
                        }

                        // TODO #184 optimize and separate outgoing/non-outgoing sockets for endpoint
                        // only yield outgoing sockets
                        // if a non-outgoing socket is found, all following sockets
                        // will also be non-outgoing
                        if !socket.can_send() {
                            info!(
                                "Socket {} is not outgoing for endpoint {}. Skipping...",
                                socket.uuid, endpoint
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
    pub fn find_known_endpoint_socket(
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
            // TODO #185: how to handle broadcasts?
            EndpointInstance::All => {
                core::todo!("#186 Undescribed by author.")
            }
        }
    }

    /// Finds the best socket over which to send a block to an endpoint.
    /// If a known socket is found, it is returned, otherwise the default socket is returned, if it
    /// exists and is not excluded.
    fn find_best_endpoint_socket(
        &self,
        local_endpoint: &Endpoint,
        endpoint: &Endpoint,
        exclude_sockets: &[ComInterfaceSocketUUID],
    ) -> Option<ComInterfaceSocketUUID> {
        // if the endpoint is the same as the hub endpoint, try to find an interface
        // that redirects @@local
        if endpoint == local_endpoint
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
            for (socket_uuid, _, _) in self.fallback_sockets.iter() {
                let socket = self.socket_by_uuid(socket_uuid);
                info!(
                    "{}: Find best for {}: {} ({}); excluded:{}",
                    local_endpoint,
                    endpoint,
                    socket_uuid,
                    socket
                        .try_lock()
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
    pub fn get_outbound_receiver_groups(
        &self,
        // TODO #187: do we need the block here for additional information (match conditions),
        // otherwise receivers are enough
        local_endpoint: &Endpoint,
        receiver_endpoints: &Vec<Endpoint>,
        mut exclude_sockets: Vec<ComInterfaceSocketUUID>,
    ) -> Option<Vec<(Option<ComInterfaceSocketUUID>, Vec<Endpoint>)>> {
        if !receiver_endpoints.is_empty() {
            let endpoint_sockets = receiver_endpoints
                .iter()
                .map(|endpoint: &Endpoint| {
                    // add sockets from endpoint blacklist
                    if let Some(blacklist) =
                        self.endpoint_sockets_blacklist.get(endpoint)
                    {
                        exclude_sockets.extend(blacklist.iter().cloned());
                    }
                    let socket = self.find_best_endpoint_socket(
                        local_endpoint,
                        endpoint,
                        &exclude_sockets,
                    );
                    (socket, endpoint)
                })
                .chunk_by(|(socket, _)| socket.clone())
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
    }

    pub fn handle_socket_event(
        &mut self,
        event: ComInterfaceSocketEvent,
        priority: InterfacePriority, // FIXME this is ugly, find a better way
    ) {
        match event {
            ComInterfaceSocketEvent::NewSocket(socket) => {
                self.handle_new_socket(socket, priority).unwrap(); // TODO: handle result
            }
            ComInterfaceSocketEvent::RemovedSocket(socket_uuid) => {
                self.delete_socket(&socket_uuid);
            }
            ComInterfaceSocketEvent::RegisteredSocket(
                socket_uuid,
                distance,
                endpoint,
            ) => {
                let socket = self.socket_by_uuid(&socket_uuid);
                self.register_socket_endpoint(socket, endpoint.clone(), distance)
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to register socket {socket_uuid} for endpoint {endpoint} {e:?}"
                    );
                });
            }
        }
    }
}