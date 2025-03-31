use crate::stdlib::collections::VecDeque;
use crate::stdlib::{cell::RefCell, rc::Rc};
use itertools::Itertools;
use log::{error, info};
use std::cell::Ref;
use std::collections::{HashMap, HashSet};
// FIXME no-std

use super::com_interfaces::com_interface::ComInterfaceError;
use super::com_interfaces::{
    com_interface::ComInterface, com_interface_socket::ComInterfaceSocket,
};
use crate::datex_values::{Endpoint, EndpointInstance};
use crate::global::dxb_block::DXBBlock;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::runtime::Context;

struct DynamicEndpointProperties {
    pub known_since: u64,
    pub distance: u32,
}

pub struct ComHub {
    pub interfaces: HashMap<ComInterfaceUUID, Rc<RefCell<dyn ComInterface>>>,
    /// a list of all available sockets, keyed by their UUID
    /// contains the socket itself and a list of endpoints currently associated with it
    // TODO: keep socket mapping up to date
    pub sockets: HashMap<
        ComInterfaceSocketUUID,
        (Rc<RefCell<ComInterfaceSocket>>, HashSet<Endpoint>),
    >,
    /// a list of all available sockets for each endpoint, with additional
    /// DynamicEndpointProperties metadata
    endpoint_sockets: HashMap<
        Endpoint,
        HashMap<ComInterfaceSocketUUID, DynamicEndpointProperties>,
    >,
    pub incoming_blocks: Rc<RefCell<VecDeque<Rc<DXBBlock>>>>,
    pub context: Rc<RefCell<Context>>,
    pub default_socket: Option<ComInterfaceSocketUUID>,
}

#[derive(Debug, Clone, Default)]
struct EndpointIterateOptions {
    pub only_direct: bool,
    pub only_outgoing: bool,
    pub exact_instance: bool,
    pub exclude_socket: Option<ComInterfaceSocketUUID>,
}

impl Default for ComHub {
    fn default() -> Self {
        ComHub {
            interfaces: HashMap::new(),
            endpoint_sockets: HashMap::new(),
            context: Rc::new(RefCell::new(Context::default())),
            incoming_blocks: Rc::new(RefCell::new(VecDeque::new())),
            sockets: HashMap::new(),
            default_socket: None,
        }
    }
}

#[derive(Debug)]
pub enum ComHubError {
    InterfaceError(ComInterfaceError),
    InterfaceAlreadyExists,
}

#[derive(Debug)]
pub enum SocketEndpointRegistrationError {
    SocketDisconnected,
    SocketUninitialized,
}

impl ComHub {
    pub fn new(context: Rc<RefCell<Context>>) -> Rc<RefCell<ComHub>> {
        Rc::new(RefCell::new(ComHub {
            context,
            ..ComHub::default()
        }))
    }

    pub fn add_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> Result<(), ComHubError> {
        let uuid = interface.borrow().get_uuid();
        if self.interfaces.contains_key(&uuid) {
            return Err(ComHubError::InterfaceAlreadyExists);
        }

        interface
            .borrow_mut()
            .connect()
            .map_err(ComHubError::InterfaceError)?;
        self.interfaces.insert(uuid, interface);

        Ok(())
    }

    pub fn remove_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> bool {
        self.interfaces
            .remove(&interface.borrow().get_uuid())
            .is_some()
    }

    pub(crate) fn receive_block(
        &self,
        block: &DXBBlock,
        socket: &RefCell<ComInterfaceSocket>,
    ) {
        info!("Received block: {:?}", block);

        // TODO: routing

        // own incoming blocks
        let mut incoming_blocks = self.incoming_blocks.borrow_mut();
        incoming_blocks.push_back(Rc::new(block.clone()));
    }

    /// registers a new endpoint that is reachable over the socket
    /// if the socket is not already registered, it will be added to the socket list
    /// if the provided endpoint is not the same as the socket endpoint,
    /// it is registered as an indirect socket to the endpoint
    pub fn register_socket_endpoint(
        &mut self,
        socket: Rc<RefCell<ComInterfaceSocket>>,
        endpoint: Endpoint,
    ) -> Result<(), SocketEndpointRegistrationError> {
        let socket_ref = socket.borrow();
        // if the registered endpoint is the same as the socket endpoint,
        // this is a direct socket to the endpoint
        let is_direct = socket_ref.endpoint == Some(endpoint.clone());

        // cannot register endpoint if socket is not connected
        if !socket_ref.is_connected {
            return Err(SocketEndpointRegistrationError::SocketDisconnected);
        }

        // cannot register endpoint if socket is not initialized (no endpoint assigned)
        if socket_ref.endpoint.is_none() {
            return Err(SocketEndpointRegistrationError::SocketUninitialized);
        }

        // TODO: set as default socket if interface is registered as default interface

        // add endpoint to socket endpoint list
        self.add_socket_endpoint(socket.clone(), endpoint.clone());

        // add socket to endpoint socket list
        self.add_endpoint_socket(&endpoint, socket_ref.uuid.clone());

        // resort sockets for endpoint
        self.sort_sockets(&endpoint);

        Ok(())
    }

    fn add_endpoint_socket(
        &mut self,
        endpoint: &Endpoint,
        socket_uuid: ComInterfaceSocketUUID,
    ) {
        if !self.endpoint_sockets.contains_key(&endpoint) {
            self.endpoint_sockets
                .insert(endpoint.clone(), HashMap::new());
        }

        let endpoint_sockets = self.endpoint_sockets.get_mut(endpoint).unwrap();
        endpoint_sockets.insert(
            socket_uuid,
            DynamicEndpointProperties {
                known_since: 0,
                distance: 0,
            },
        );
    }

    fn add_socket_endpoint(
        &mut self,
        socket: Rc<RefCell<ComInterfaceSocket>>,
        endpoint: Endpoint,
    ) {
        if !self.sockets.contains_key(&socket.borrow().uuid) {
            self.sockets.insert(
                socket.borrow().uuid.clone(),
                (socket.clone(), HashSet::new()),
            );
        }

        // add endpoint to socket endpoint list
        self.sockets
            .get_mut(&socket.borrow().uuid)
            .unwrap()
            .1
            .insert(endpoint.clone());
    }

    /// Sort the sockets for an endpoint:
    /// - direct sockets first
    /// - then sort by channel channel_factor (latency, bandwidth)
    /// - then sort by socket connect_timestamp
    fn sort_sockets(&mut self, endpoint: &Endpoint) {
        let sockets = self.endpoint_sockets.get_mut(endpoint).unwrap();

        // TODO: implement sorting logic
        /*let sorted_sockets =

        self.endpoint_sockets.insert(
            endpoint.clone(),
            sorted_sockets,
        );*/
    }

    fn get_socket_by_uuid(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
    ) -> Rc<RefCell<ComInterfaceSocket>> {
        self.sockets
            .get(socket_uuid)
            .map(|socket| socket.0.clone())
            .unwrap_or_else(|| {
                panic!("Socket for uuid {} not found", socket_uuid)
            })
    }

    /// Iterate over all sockets of all interfaces
    #[deprecated(note = "Iterate self.sockets instead")]
    fn iterate_all_sockets(&self) -> Vec<Rc<RefCell<ComInterfaceSocket>>> {
        let mut sockets = Vec::new();
        for interface in self.interfaces.values() {
            let interface_ref = interface.borrow();
            for socket in interface_ref.get_sockets().borrow().iter() {
                sockets.push(socket.clone());
            }
        }
        sockets.clone()
    }

    fn get_socket_interface_properties(
        interfaces: &HashMap<ComInterfaceUUID, Rc<RefCell<dyn ComInterface>>>,
        interface_uuid: &ComInterfaceUUID,
    ) -> InterfaceProperties {
        interfaces
            .get(interface_uuid)
            .unwrap()
            .borrow()
            .get_properties()
    }

    fn iterate_endpoint_sockets<'a>(
        &'a self,
        endpoint: &'a Endpoint,
        options: EndpointIterateOptions,
    ) -> impl Iterator<Item = ComInterfaceSocketUUID> + 'a {
        let endpoint_sockets = self.endpoint_sockets.get(endpoint);
        let interfaces = &self.interfaces;

        std::iter::from_coroutine(
            #[coroutine]
            move || {
                for socket_uuid in endpoint_sockets.unwrap().keys() {
                    {
                        let socket = self.get_socket_by_uuid(socket_uuid);
                        let socket = socket.borrow();
                        // check if is direct socket if only_redirect is set to true
                        if !options.only_direct
                            && match &socket.endpoint {
                                Some(e) => e == endpoint,
                                _ => false,
                            }
                        {
                            continue;
                        }

                        // check if the socket is excluded if exclude_socket is set
                        if let Some(exclude_socket) = &options.exclude_socket {
                            if socket.uuid == *exclude_socket {
                                continue;
                            }
                        }

                        // check if the socket is outgoing if only_outgoing is set to true
                        let properties =
                            ComHub::get_socket_interface_properties(
                                interfaces,
                                &socket.interface_uuid,
                            );
                        if options.only_outgoing && !properties.can_send() {
                            continue;
                        }
                    }

                    yield socket_uuid.clone()
                }
            },
        )
    }

    /// Finds the best matching socket over which an endpoint is known to be reachable.
    fn find_known_endpoint_socket(
        &self,
        endpoint: &Endpoint,
        exclude_socket: Option<ComInterfaceSocketUUID>,
    ) -> Option<ComInterfaceSocketUUID> {
        match endpoint.instance {
            // find socket for any endpoint instance
            EndpointInstance::Any => {
                let options = EndpointIterateOptions {
                    only_direct: false,
                    only_outgoing: true,
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
                    only_outgoing: true,
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
        exclude_socket: Option<ComInterfaceSocketUUID>,
    ) -> Option<ComInterfaceSocketUUID> {
        // find best known socket for endpoint
        let matching_socket =
            self.find_known_endpoint_socket(endpoint, exclude_socket.clone());

        // if a matching socket is found, return it
        if matching_socket.is_some() {
            matching_socket
        }
        // otherwise, return the default socket if it exists and is not excluded
        else {
            if self.default_socket.is_some()
                && (exclude_socket.is_none()
                    || self.default_socket.clone().unwrap()
                        != exclude_socket.clone().unwrap())
            {
                Some(self.default_socket.clone().unwrap())
            } else {
                None
            }
        }
    }

    /// returns all receivers to which the block has to be sent, grouped by the
    /// outbound socket
    fn get_outbound_receiver_groups(
        &self,
        block: &DXBBlock,
        exclude_socket: Option<ComInterfaceSocketUUID>,
    ) -> Option<Vec<(Option<ComInterfaceSocketUUID>, Vec<Endpoint>)>> {
        if let Some(receivers) = block.receivers() {
            if receivers.len() != 0 {
                let endpoint_sockets = receivers
                    .iter()
                    .map(|e| {
                        let socket = self.find_best_endpoint_socket(
                            e,
                            exclude_socket.clone(),
                        );
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

    /// Update all sockets and interfaces,
    /// collecting incoming data and sending out queued blocks.
    pub fn update(&mut self) {
        // update sockets
        self.update_sockets();

        // receive blocks from all sockets
        self.receive_incoming_blocks();

        // send all queued blocks from all interfaces
        self.flush_outgoing_blocks();
    }

    /// Send a block to all endpoints specified in the block header.
    /// The routing algorithm decides which sockets are used to send the block, based on the endpoint.
    /// A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
    /// The original_socket parameter is used to prevent sending the block back to the sender.
    /// When this method is called, the block is queued in the send queue.
    pub fn send_block(
        &self,
        block: &DXBBlock,
        original_socket: Option<ComInterfaceSocketUUID>,
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
                error!("Cannot send block, no receiver sockets found");
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
        let mut socket_ref = socket.borrow_mut();
        match &addressed_block.to_bytes() {
            Ok(bytes) => {
                // TODO: resend block if socket failed to send
                socket_ref.queue_outgoing_block(&bytes);
            }
            Err(err) => {
                error!("Failed to convert block to bytes: {:?}", err);
            }
        }
    }

    fn update_sockets(&self) {
        // update sockets, collect incoming data into full blocks
        info!("Collecting incoming data from all sockets");
        for socket in &self.iterate_all_sockets() {
            let mut socket_ref = socket.borrow_mut();
            socket_ref.collect_incoming_data();
        }
    }

    /// Collect all blocks from the receive queues of all sockets and process them
    /// in the receive_block method.
    fn receive_incoming_blocks(&mut self) {
        // iterate over all sockets
        for socket in &self.iterate_all_sockets() {
            let socket_ref = socket.borrow();
            let block_queue = socket_ref.get_incoming_block_queue();
            for block in block_queue {
                self.receive_block(block, socket);
            }
        }
    }

    /// Send all queued blocks from all interfaces.
    fn flush_outgoing_blocks(&mut self) {
        for interface in self.interfaces.values() {
            interface.borrow_mut().flush_outgoing_blocks();
        }
    }
}
