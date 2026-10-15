//! This module contains the implementation of the Communication Hub (ComHub) for DATEX, which is responsible for managing network communication.
use crate::{
    channel::mpsc::UnboundedSender,
    collections::HashMap,
    global::protocol_structures::{
        block_header::BlockType, routing_header::SignatureType,
    },
    network::com_hub::{
        errors::{ComHubError, SocketEndpointRegistrationError},
        managers::com_interface_manager::ComInterfaceManager,
        network_response::{
            Response, ResponseError, ResponseOptions,
            ResponseResolutionStrategy,
        },
        options::ComHubOptions,
    },
    task,
    utils::maybe_async::SyncOrAsyncResolved,
};

use crate::prelude::*;

pub mod managers;

pub mod metadata;
use crate::network::com_hub::managers::socket_manager::{
    ComInterfaceSocketManager, SocketCloseReceiver, SocketData,
};

pub mod errors;
pub mod network_response;

pub mod network_tracing;
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use core::{
    cell::RefCell,
    cmp::PartialEq,
    fmt::{Debug, Formatter},
    panic,
    pin::Pin,
    result::Result,
};
use itertools::Itertools;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

pub mod options;
use crate::{
    global::dxb_block::{DXBBlock, IncomingSection},
    network::{
        block_handler::{BlockHandler, BlockHistoryData},
        com_hub::network_tracing::{
            NetworkTraceHop, NetworkTraceHopDirection, NetworkTraceHopSocket,
        },
    },
    values::core_values::endpoint::Endpoint,
};
pub mod com_hub_interface;
mod com_hub_socket;
#[cfg(test)]
pub mod test_utils;

use crate::{
    collections::HashSet,
    crypto::CryptoImpl,
    global::dxb_block::{BlockId, SignatureValidationError},
    network::{
        com_hub::managers::com_interface_manager::InterfaceCloseReceiver,
        com_interfaces::{
            block_collector::BlockCollector,
            com_interface::{
                ComInterfaceUUID,
                factory::{
                    CloseAsyncCallback, ComInterfaceConfiguration,
                    SendCallback, SendFailure, SendSuccess, SocketDataIterator,
                    SocketProperties,
                },
                properties::InterfaceDirection,
            },
        },
    },
    time::now_ms,
    utils::{
        async_iterators::async_next_pin_box,
        maybe_async::{MaybeAsync, SyncOrAsync, SyncOrAsyncResult},
        task_manager::TaskManager,
    },
};
use async_select::select;
use datex_crypto_facade::crypto::Crypto;
use datex_macros_internal::Datex;
use futures::channel::{oneshot, oneshot::Sender};
use futures_util::FutureExt;

pub type IncomingBlockInterceptor =
    Box<dyn Fn(&DXBBlock, &ComInterfaceSocketUUID) + 'static>;

pub type OutgoingBlockInterceptor =
    Box<dyn Fn(&DXBBlock, &ComInterfaceSocketUUID, &[Endpoint]) + 'static>;

pub struct ComHub {
    /// the runtime endpoint of the hub (@me)
    pub endpoint: Endpoint,

    /// ComHub configuration options
    pub options: ComHubOptions,

    socket_manager: ComInterfaceSocketManager,
    interfaces_manager: ComInterfaceManager,

    pub block_handler: BlockHandler,

    incoming_block_interceptors: RefCell<Vec<IncomingBlockInterceptor>>,
    outgoing_block_interceptors: RefCell<Vec<OutgoingBlockInterceptor>>,

    pub task_manager: TaskManager,
}

impl Debug for ComHub {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ComHub")
            .field("endpoint", &self.endpoint)
            .field("options", &self.options)
            .finish()
    }
}

#[derive(
    Datex,
    Debug,
    Clone,
    Copy,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[datex(structural)]
/// The priority of an interface, which determines the order in which interfaces are used for routing fallback logic.
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

impl From<Option<u16>> for InterfacePriority {
    fn from(value: Option<u16>) -> Self {
        match value {
            Some(priority) => InterfacePriority::Priority(priority),
            None => InterfacePriority::default(),
        }
    }
}

impl Default for InterfacePriority {
    fn default() -> Self {
        InterfacePriority::Priority(0)
    }
}

pub struct ReceiveBlockPreprocessResult {
    relayed_block: Option<DXBBlock>,
    own_received_block: Option<DXBBlock>,
    block_id_for_history: Option<BlockId>,
    is_for_own: bool,
}

pub type BlockSendSyncOrAsyncResult<F> =
    SyncOrAsyncResult<Option<Vec<Vec<u8>>>, (), Vec<Endpoint>, F>;

pub type PrepareOwnBlockFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DXBBlock, ComHubError>> + 'a>>;

pub type PrepareOwnBlockResult<'a> = SyncOrAsyncResult<
    DXBBlock,
    DXBBlock,
    ComHubError,
    PrepareOwnBlockFuture<'a>,
>;

pub type ReceiveBlockResult =
    Result<Option<DXBBlock>, SignatureValidationError>;

impl ComHub {
    pub fn create(
        endpoint: impl Into<Endpoint>,
        incoming_sections_sender: UnboundedSender<IncomingSection>,
    ) -> (Rc<ComHub>, impl Future<Output = ()>) {
        let (task_manager, task_future) = TaskManager::create();

        let block_handler = BlockHandler::init(incoming_sections_sender);
        let com_hub = Rc::new(ComHub {
            endpoint: endpoint.into(),
            options: ComHubOptions::default(),
            block_handler,
            socket_manager: ComInterfaceSocketManager::new(),
            interfaces_manager: ComInterfaceManager::default(),
            incoming_block_interceptors: RefCell::new(Vec::new()),
            outgoing_block_interceptors: RefCell::new(Vec::new()),
            task_manager,
        });

        (com_hub, task_future)
    }

    /// Registers the handle_sockets_task for the given ComInterfaceConfiguration
    pub(crate) fn register_com_interface_handler(
        self: Rc<Self>,
        com_interface_configuration: ComInterfaceConfiguration,
        priority: InterfacePriority,
        close_receiver: InterfaceCloseReceiver,
    ) -> Option<impl Future<Output = Result<(), ()>>> {
        let (socket_ready_senders, socket_ready_fut) =
            if com_interface_configuration.has_single_socket {
                let (r, f) = ComHub::get_ready_senders();
                (Some(r), Some(f))
            } else {
                (None, None)
            };

        self.task_manager
            .register_task(self.clone().handle_sockets_task(
                com_interface_configuration,
                priority,
                close_receiver,
                socket_ready_senders,
            ));
        socket_ready_fut
    }

    /// Sets up two oneshot receivers:
    /// The first one should resolve once the socket is fully connected, meaning the peer endpoint of the socket is known
    /// The second one should resolve once the local endpoint has sent a Hello block
    /// Returns a future that resolves once both oneshot receivers have resolved
    fn get_ready_senders() -> (
        (Sender<Result<(), ()>>, Sender<Result<(), ()>>),
        impl Future<Output = Result<(), ()>>,
    ) {
        let (socket_ready_sender, socket_ready_receiver) = oneshot::channel();
        let (socket_hello_sent_sender, socket_hello_sent_receiver) =
            oneshot::channel();
        let fut = async move {
            match socket_ready_receiver.await {
                Ok(Ok(())) => match socket_hello_sent_receiver.await {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(())) => Err(()),
                    Err(_) => Err(()),
                },
                Ok(Err(())) => Err(()),
                Err(_) => Err(()),
            }
        };
        ((socket_ready_sender, socket_hello_sent_sender), fut)
    }

    /// Iterates over the given NewSocketsIterator for an interface and handles each socket
    async fn handle_sockets_task(
        self: Rc<Self>,
        com_interface_configuration: ComInterfaceConfiguration,
        interface_priority: InterfacePriority,
        interface_close_receiver: InterfaceCloseReceiver,
        ready_senders: Option<(Sender<Result<(), ()>>, Sender<Result<(), ()>>)>,
    ) {
        let com_interface_uuid = com_interface_configuration.uuid();
        let mut iterator = com_interface_configuration.new_sockets_iterator;
        let com_interface_properties =
            com_interface_configuration.properties.clone();
        let cleanup_callback = com_interface_configuration.close_async_callback;

        let (mut socket_ready_sender, mut hello_sent_sender) =
            match ready_senders {
                Some((x, y)) => (Some(x), Some(y)),
                None => (None, None),
            };

        let closed_sender = select!(
            _ = async {
                while let Some(socket) = async_next_pin_box(&mut iterator).await {

                    if !self.interfaces_manager.has_interface(&com_interface_uuid) {
                        info!("Interface {} was removed while waiting for new socket connections, stopping socket handler task", com_interface_uuid);
                        break;
                    }

                    match socket {
                        Ok(socket_configuration) => {
                            let socket_iterator = socket_configuration.iterator;
                            let send_callback = socket_configuration.send_callback;
                            let cleanup_callback = socket_configuration.close_async_callback;
                            let socket_properties = socket_configuration.properties;
                            let _socket_uuid = socket_properties.uuid();
                            let _socket_direction = socket_properties.direction.clone();
                            let (socket_close_sender, socket_close_receiver) = oneshot::channel();

                            // store socket info
                            let _res = self.socket_manager.register_socket(
                                SocketData {
                                    socket_properties: socket_properties.clone(),
                                    interface_uuid: com_interface_uuid.clone(),
                                    interface_properties: com_interface_properties
                                        .clone(),
                                    send_callback,
                                    endpoints: HashSet::new(),
                                    close_sender: Some(socket_close_sender),
                                    socket_ready_sender: socket_ready_sender.take(),
                                },
                                interface_priority,
                            );
                            // TODO #729: handle error

                            let self_clone = self.clone();
                            if let Some(socket_iterator) = socket_iterator {
                                self_clone.task_manager.register_task(
                                    self_clone.clone().handle_socket_task(
                                        socket_properties,
                                        socket_iterator,
                                        com_interface_uuid.clone(),
                                        com_interface_properties.auto_identify,
                                        socket_close_receiver,
                                        cleanup_callback,
                                        hello_sent_sender.take()
                                    ),
                                );
                            }
                        }
                        Err(e) => {
                            error!("Error creating socket from iterator: {:?}", e);
                            break;
                        }
                    }
                }
            } => {
                None
            },
            sender = interface_close_receiver => {
                Some(sender.unwrap())
            }
        );

        // call cleanup callback for interface if defined
        if let Some(cleanup_callback) = cleanup_callback {
            cleanup_callback().await;
        }

        // only if interface still exists
        if self.interfaces_manager.has_interface(&com_interface_uuid) {
            // indicate that interface is no longer waiting for new socket connections (e.g. for single socket interfaces)
            self.interfaces_manager
                .set_interface_waiting_for_socket_connections(
                    &com_interface_uuid,
                    false,
                );

            // if interface has no sockets, it can be destroyed
            if !self
                .socket_manager
                .are_sockets_registered_for_interface(&com_interface_uuid)
            {
                self.interfaces_manager
                    .cleanup_interface(&com_interface_uuid)
                    .unwrap();
                info!(
                    "Destroyed interface {} as it has no sockets registered",
                    com_interface_uuid
                );
            }
        }

        if let Some(closed_sender) = closed_sender {
            closed_sender.send(()).unwrap();
        }

        // if socket_ready_sender is still Some, socket was never connected, send Error
        if let Some(socket_ready_sender) = socket_ready_sender {
            let _ = socket_ready_sender.send(Err(()));
        }
        // if hello_sent_sender is still Some, hello was never sent, send Error
        if let Some(hello_sent_sender) = hello_sent_sender {
            let _ = hello_sent_sender.send(Err(()));
        }
    }

    /// Sends a hello block via the given socket if the socket direction allows sending and auto_identify is enabled for the interface
    async fn send_socket_hello(
        self: Rc<Self>,
        socket_uuid: ComInterfaceSocketUUID,
        socket_direction: InterfaceDirection,
        auto_identify: bool,
        hello_sent_sender: Option<Sender<Result<(), ()>>>,
    ) {
        let send_hello = socket_direction.can_send() && auto_identify; // Only send hello if auto_identify is enabled

        if send_hello {
            info!("Saying hello to {}", socket_uuid);
            if let Err(err) = self.send_hello_block(socket_uuid).await {
                error!("Failed to send hello block: {:?}", err);
                hello_sent_sender.map(|sender| sender.send(Err(())));
            } else {
                hello_sent_sender.map(|sender| sender.send(Ok(())));
            }
        } else {
            hello_sent_sender.map(|sender| sender.send(Ok(())));
        }
    }

    /// Handles incoming data from the given SocketDataIterator
    async fn handle_socket_task(
        self: Rc<Self>,
        socket_properties: SocketProperties,
        mut socket_iterator: SocketDataIterator,
        com_interface_uuid: ComInterfaceUUID,
        auto_identify: bool,
        close_receiver: SocketCloseReceiver,
        cleanup_callback: Option<CloseAsyncCallback>,
        hello_sent_sender: Option<Sender<Result<(), ()>>>,
    ) {
        // send hello block in background task
        self.task_manager
            .register_task(self.clone().send_socket_hello(
                socket_properties.uuid(),
                socket_properties.direction.clone(),
                auto_identify,
                hello_sent_sender,
            ));

        let (mut bytes_sender, block_iterator) = BlockCollector::create();
        let mut block_iterator = Box::pin(block_iterator);

        let closed_sender = select!(
            _ = async {
                loop {
                    select! {
                        // receive new block data from socket
                        data = async_next_pin_box(&mut socket_iterator).fuse() => {
                            match data {

                                // next data block
                                Some(Ok(data)) => {
                                    // send data to block collector
                                    if let Err(e) = bytes_sender.start_send(data) {
                                        error!("Error sending data to BlockCollector: {:?}", e);
                                        break;
                                    }
                                }

                                // got error
                                Some(Err(_)) => {
                                    error!("Socket {} closed, removing socket", socket_properties.uuid());
                                    break;
                                }

                                // no more data, gracefull exit
                                None => {
                                    error!("Socket {} closed (iterator finished), removing socket", socket_properties.uuid());
                                    break;
                                }
                            }
                        },
                        // receive new blocks from block collector
                        Some(block) = async_next_pin_box(&mut block_iterator).fuse() => {
                            // spawn as separate task to handle incoming block
                            // this improves performance as multiple blocks can be handled in parallel
                            // it's also required because the size of this future gets to big for embedded targets (heap allocation fails)
                            self.clone().handle_incoming_block_async(block, socket_properties.uuid()).await;
                        },

                        complete => break,
                    }
                }
            } => None,
            closed_sender = close_receiver => {
                info!("received socket close signal for socket removing socket");
                Some(closed_sender.unwrap())
            }
        );

        // call cleanup callback for socket if defined
        if let Some(cleanup_callback) = cleanup_callback {
            cleanup_callback().await;
        }

        // socket closed, remove
        self.socket_manager
            .cleanup_socket(&socket_properties.uuid());

        // only if interface still exists
        if self.interfaces_manager.has_interface(&com_interface_uuid) {
            // TODO #730: check if any other sockets are still registered for the interface, if not
            // and if interface is no longer waiting for new socket connections (e.g. single socket interface), also remove the interface
            if !self
                .interfaces_manager
                .is_interface_waiting_for_socket_connections(
                    &com_interface_uuid,
                )
            {
                self.interfaces_manager
                    .cleanup_interface(&com_interface_uuid)
                    .unwrap();
                info!(
                    "Destroyed interface {} as it is no longer waiting for socket connections",
                    com_interface_uuid
                );
                // TODO #731: reconnect logic?
            }
        }

        if let Some(closed_sender) = closed_sender {
            closed_sender.send(()).unwrap();
        }
    }

    /// Handles an incoming block from a socket (async)
    async fn handle_incoming_block_async(
        self: Rc<Self>,
        block: DXBBlock,
        socket_uuid: ComInterfaceSocketUUID,
    ) {
        // handle incoming block
        let receive_block_result = self
            .clone()
            .receive_block(block, socket_uuid)
            .into_future()
            .await;

        let own_received_block = match receive_block_result {
            Ok(own_received_block) => own_received_block,
            Err(e) => {
                error!("Failed to validate block signature: {:?}", e);
                return;
            }
        };

        // handle own block if some
        if let Some(own_received_block) = own_received_block {
            self.block_handler.handle_incoming_block(own_received_block);
        }
    }

    /// Checks if the given endpoint is the local endpoint, matching instances as well
    pub fn is_local_endpoint_exact(&self, endpoint: &Endpoint) -> bool {
        self.endpoint.is_local_or_equals_endpoint(endpoint)
    }

    /// Register an incoming block interceptor
    pub fn register_incoming_block_interceptor<F>(&self, interceptor: F)
    where
        F: Fn(&DXBBlock, &ComInterfaceSocketUUID) + 'static,
    {
        self.incoming_block_interceptors
            .borrow_mut()
            .push(Box::new(interceptor));
    }

    /// Register an outgoing block interceptor
    pub fn register_outgoing_block_interceptor<F>(&self, interceptor: F)
    where
        F: Fn(&DXBBlock, &ComInterfaceSocketUUID, &[Endpoint]) + 'static,
    {
        self.outgoing_block_interceptors
            .borrow_mut()
            .push(Box::new(interceptor));
    }

    /// Receives a block from a socket and handles it accordingly
    /// Returns a MaybeAsync which is async in the following cases:
    /// * the block signature is validated because the block is for own endpoint and signature is set
    /// * the block needs to be relayed to other endpoints or is a trace block, which requires async handling for the relay/trace logic
    ///   The MaybeAsync returns the own received block if the block is for own endpoint and signature is valid, otherwise None
    // FIXME #732: this seams to generate a very big future which causes heap allocation to fail on embedded targets (!?)
    pub(crate) fn receive_block(
        self: Rc<Self>,
        block: DXBBlock,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> MaybeAsync<ReceiveBlockResult, impl Future<Output = ReceiveBlockResult>>
    {
        // preprocess the block and get relay receivers and own received block if any
        let preprocess_result =
            self.receive_block_preprocess(&socket_uuid, block);

        let self_clone = self.clone();

        // validate block signature if sent to own endpoint
        let validation_result = match preprocess_result.own_received_block {
            // if block is for own endpoint, validate signature if set
            Some(block) => block
                .validate_signature()
                .map(|validation| validation.map(Some)),
            // if block is not for own endpoint, don't validate signature and don't return own received block
            None => MaybeAsync::Sync(Ok(None)),
        };

        validation_result
            .map(move |validation_result| {
                // handle async logic for received blocks if needed
                let own_received_block = match validation_result {
                    Ok(block) => block,
                    Err(e) => return MaybeAsync::Sync(Err(e)),
                };

                let (trace_block, own_block) = match own_received_block {
                    Some(block) => {
                        let block_type = block.block_type();
                        match block_type {
                            BlockType::Trace | BlockType::TraceBack => {
                                (Some(block), None)
                            }
                            _ => (None, Some(block)),
                        }
                    }
                    None => (None, None),
                };

                if preprocess_result.relayed_block.is_some()
                    || trace_block.is_some()
                {
                    MaybeAsync::Async(async move {
                        self_clone
                            .receive_block_async(
                                trace_block,
                                preprocess_result.relayed_block,
                                preprocess_result.block_id_for_history,
                                socket_uuid,
                                preprocess_result.is_for_own,
                            )
                            .await;

                        Ok(own_block)
                    })
                }
                // otherwise, return directly without async handler
                else {
                    MaybeAsync::Sync(Ok(own_block))
                }
            })
            .flatten()
    }

    /// Preprocesses a received block and returns relay receivers and own received block if any
    fn receive_block_preprocess(
        &self,
        socket_uuid: &ComInterfaceSocketUUID,
        block: DXBBlock,
    ) -> ReceiveBlockPreprocessResult {
        info!("{} received block: {}", self.endpoint, block);

        for interceptor in self.incoming_block_interceptors.borrow().iter() {
            interceptor(&block, socket_uuid);
        }

        let block_type = block.block_header.flags_and_timestamp.block_type();

        // register in block history
        let is_new_block = !self.block_handler.is_block_in_history(&block);

        // assign endpoint to socket if none is assigned
        // only if a new block and the sender in not the local endpoint
        if is_new_block
            && !self.is_local_endpoint_exact(&block.routing_header.sender)
        {
            self.register_socket_endpoint_from_incoming_block(
                socket_uuid.clone(),
                &block,
            );
        }

        let all_receivers = block.receiver_endpoints();
        let (relayed_block, own_received_block, is_for_own) =
            if !all_receivers.is_empty() {
                let is_for_own = all_receivers.iter().any(|e| {
                    self.is_local_endpoint_exact(e)
                        || e == &Endpoint::ANY
                        || e == &Endpoint::ANY_ALL_INSTANCES
                });

                // handle blocks for own endpoint
                let own_received_block =
                    if is_for_own && block_type != BlockType::Hello {
                        info!("Block is for this endpoint ({})", self.endpoint);

                        Some(block.clone()) // FIXME #733: no clone
                    } else {
                        None
                    };

                // TODO #177: handle this via TTL, not explicitly for Hello blocks
                let relay_receivers = {
                    let should_relay =
                    // don't relay "Hello" blocks sent to own endpoint
                    !(
                        is_for_own && block_type == BlockType::Hello
                    );

                    // relay the block to other endpoints
                    if should_relay {
                        let relay_receivers = if is_for_own {
                            // get all receivers that the block must be relayed to
                            self.get_remote_receivers(&all_receivers)
                        } else {
                            all_receivers
                        };
                        if relay_receivers.is_empty() {
                            None
                        } else {
                            Some(relay_receivers)
                        }
                    } else {
                        None
                    }
                };

                let relayed_block = relay_receivers
                    .map(|receivers| block.clone_with_new_receivers(receivers));

                (relayed_block, own_received_block, is_for_own)
            } else {
                (None, None, false)
            };

        // add to block history
        let block_id_for_history = if is_new_block {
            Some(block.get_block_id())
        } else {
            None
        };

        ReceiveBlockPreprocessResult {
            relayed_block,
            own_received_block,
            block_id_for_history,
            is_for_own,
        }
    }

    /// Handles async logic for received blocks (trace blocks, redirects to other endpoints)
    pub(crate) async fn receive_block_async(
        self: Rc<Self>,
        trace_block: Option<DXBBlock>,
        relayed_block: Option<DXBBlock>,
        block_id_for_history: Option<BlockId>,
        socket_uuid: ComInterfaceSocketUUID,
        is_for_own: bool,
    ) {
        // handle trace block asynchronously
        if let Some(block) = trace_block {
            info!("Handling trace block asynchronously");

            match block.block_type() {
                BlockType::Trace => {
                    self.handle_trace_block(&block, socket_uuid.clone()).await;
                }
                BlockType::TraceBack => {
                    self.handle_trace_back_block(&block, socket_uuid.clone());
                }
                _ => unreachable!(), // not a trace block, should never happen
            }
        }

        // redirect block to other endpoints
        if let Some(block) = relayed_block {
            match block.block_type() {
                BlockType::Trace | BlockType::TraceBack => {
                    self.redirect_trace_block(
                        block,
                        socket_uuid.clone(),
                        is_for_own,
                    )
                    .await;
                }
                _ => {
                    self.redirect_block(block, socket_uuid.clone(), is_for_own)
                        .await
                        .unwrap(); // TODO #734: handle error
                }
            }
        }

        // add to block history
        if let Some(block_id) = block_id_for_history {
            self.block_handler
                .add_block_id_to_history(block_id, Some(socket_uuid));
        }
    }

    /// Returns a list of all receivers from a given ReceiverEndpoints
    /// excluding the local endpoint
    fn get_remote_receivers(
        &self,
        receiver_endpoints: &[Endpoint],
    ) -> Vec<Endpoint> {
        receiver_endpoints
            .iter()
            .filter(|e| !self.is_local_endpoint_exact(e))
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
        let mut socket =
            self.socket_manager.get_socket_by_uuid_mut(&socket_uuid);

        let distance = block.routing_header.distance;
        let sender = block.routing_header.sender.clone();

        // set as direct endpoint if distance = 0
        if socket.socket_properties.direct_endpoint.is_none() && distance == 1 {
            info!(
                "Setting direct endpoint for socket {}: {}",
                socket.socket_properties.uuid(),
                sender
            );
            socket.socket_properties.direct_endpoint = Some(sender.clone());
        }
        let uuid = socket.socket_properties.uuid().clone();

        drop(socket);

        match self.socket_manager.register_socket_endpoint(
            uuid,
            sender.clone(),
            distance,
        ) {
            Err(SocketEndpointRegistrationError::SocketEndpointAlreadyRegistered) => {
                debug!(
                    "Socket already registered for endpoint {sender}",
                );
            }
            Ok(_) => { }
        }
    }

    /// Prepares a block and relays it to the given receivers.
    /// The routing distance is incremented by 1.
    pub(crate) async fn redirect_block(
        &self,
        mut block: DXBBlock,
        incoming_socket: ComInterfaceSocketUUID,
        // only for debugging traces
        forked: bool,
    ) -> Result<(), Vec<Endpoint>> {
        let receivers = block.receiver_endpoints();

        // check if block has already passed this endpoint (-> bounced back block)
        // and add to blacklist for all receiver endpoints
        let history_block_data =
            self.block_handler.get_block_data_from_history(&block);
        if history_block_data.is_some() {
            for receiver in &receivers {
                if !self.is_local_endpoint_exact(receiver) {
                    info!(
                        "{}: Adding socket {} to blacklist for receiver {}",
                        self.endpoint, incoming_socket, receiver
                    );
                    self.socket_manager.add_to_endpoint_blocklist(
                        receiver.clone(),
                        &incoming_socket,
                    );
                }
            }
        }

        // increment distance for next hop
        block.routing_header.distance += 1;

        // ensure ttl is >= 1
        // decrease TTL by 1
        if block.routing_header.ttl > 1 {
            block.routing_header.ttl -= 1;
        }
        // if ttl becomes 0 after decrement drop the block
        else if block.routing_header.ttl == 1 {
            block.routing_header.ttl -= 1;
            warn!("Block TTL expired. Dropping block...");
            return Ok(());
        // else ttl must be zero
        } else {
            warn!("Block TTL expired. Dropping block...");
            return Ok(());
        }

        let mut prefer_incoming_socket_for_bounce_back = false;
        // if we are the original sender of the block, don't send again (prevent loop) and send
        // bounce back block with all receivers
        let res = {
            if self.is_local_endpoint_exact(&block.routing_header.sender) {
                // if not bounce back block, directly send back to incoming socket (prevent loop)
                prefer_incoming_socket_for_bounce_back =
                    !block.is_bounce_back();
                Err(receivers.to_vec())
            } else {
                let mut excluded_sockets = vec![incoming_socket.clone()];
                if let Some(BlockHistoryData {
                    original_socket_uuid: Some(original_socket_uuid),
                }) = &history_block_data
                {
                    excluded_sockets.push(original_socket_uuid.clone())
                }
                self.send_block_async(block.clone(), excluded_sockets, forked)
                    .await
            }
        };

        // send block for unreachable endpoints back to the sender
        if let Err(unreachable_endpoints) = res {
            // try to send back to original socket
            // if already in history, get original socket from history
            // otherwise, directly send back to the incoming socket
            let send_back_socket = if !prefer_incoming_socket_for_bounce_back
                && let Some(history_block_data) = history_block_data
            {
                history_block_data.original_socket_uuid
            } else {
                Some(incoming_socket.clone())
            };

            // If a send_back_socket is set, the original block is not from this endpoint,
            // so we can send it back to the original socket
            if let Some(send_back_socket) = send_back_socket {
                // never send a bounce back block back again to the incoming socket
                if block.is_bounce_back() && send_back_socket == incoming_socket
                {
                    warn!(
                        "{}: Tried to send bounce back block back to incoming socket, but this is not allowed",
                        self.endpoint
                    );
                    Ok(())
                } else if let Some(socket) =
                    self.socket_manager.get_socket_by_uuid(&send_back_socket)
                    && socket.socket_properties.direction.can_send()
                {
                    block.set_bounce_back(true);
                    self
                        .send_block_to_endpoints_via_socket(
                            block,
                            send_back_socket,
                            unreachable_endpoints.clone(),
                            if forked { Some(0) } else { None },
                        )
                        .into_error_future()
                        .await
                    .map_or(Ok(()), |e| {
                        error!(
                            "{}: Failed to send bounce back block to socket: {:?}",
                            self.endpoint, e
                        );
                        Err(unreachable_endpoints)
                    })
                } else {
                    error!(
                        "Tried to send bounce back block, but cannot send back to incoming socket"
                    );
                    Err(unreachable_endpoints)
                }
            }
            // Otherwise, the block originated from this endpoint, we can just call send again
            // and try to send it via other remaining sockets that are not on the blacklist for the
            // block receiver
            else {
                self.send_block_async(block, vec![], forked).await.map_or(Ok(()), |e| {
                    error!(
                        "{}: Failed to send bounce back block to socket: {:?}",
                        self.endpoint, e
                    );
                    Err(unreachable_endpoints)
                })
            }
        } else {
            Ok(())
        }
    }

    /// Prepares an own block for sending by setting sender, timestamp, distance and signing if needed.
    /// Will return either synchronously or asynchronously depending on the signature type.
    pub fn prepare_own_block(
        &self,
        mut block: DXBBlock,
    ) -> PrepareOwnBlockResult<'_> {
        /// Updates the sender and timestamp of the block
        fn update_sender_and_timestamp(
            mut block: DXBBlock,
            endpoint: Endpoint,
        ) -> Result<DXBBlock, ComHubError> {
            let now = now_ms();
            block.routing_header.sender = endpoint;
            block
                .block_header
                .flags_and_timestamp
                .set_creation_timestamp(now);
            block.routing_header.distance = 1;
            Ok(block)
        }

        match block.routing_header.flags.signature_type() {
            // SignatureType::None can be handled synchronously
            SignatureType::None => SyncOrAsync::Sync(
                update_sender_and_timestamp(block, self.endpoint.clone()),
            ),

            // SignatureType::Unencrypted and SignatureType::Encrypted require async signing
            sig_ty => {
                let endpoint = self.endpoint.clone();

                SyncOrAsync::Async(Box::pin(async move {
                    let (pub_key, pri_key) = CryptoImpl::gen_ed25519()
                        .await
                        .map_err(|_| ComHubError::SignatureCreationError)?;

                    let raw_signed =
                        [pub_key.clone(), block.body.clone()].concat();

                    let hashed_signed = CryptoImpl::hash_sha256(&raw_signed)
                        .await
                        .map_err(|_| ComHubError::SignatureCreationError)?;

                    let signature =
                        CryptoImpl::sig_ed25519(&pri_key, &hashed_signed)
                            .await
                            .map_err(|_| ComHubError::SignatureCreationError)?;

                    let sig_bytes: Vec<u8> = match sig_ty {
                        SignatureType::Unencrypted => signature.to_vec(),

                        SignatureType::Encrypted => {
                            let hash =
                                CryptoImpl::hkdf_sha256(&pub_key, &[0u8; 16])
                                    .await
                                    .map_err(|_| {
                                        ComHubError::SignatureCreationError
                                    })?;

                            CryptoImpl::aes_ctr_encrypt(
                                &hash, &[0u8; 16], &signature,
                            )
                            .await
                            .map_err(|_| ComHubError::SignatureCreationError)?
                            .to_vec()
                        }

                        SignatureType::None => unreachable!("handled above"),
                    };

                    block.signature = Some([sig_bytes, pub_key].concat());
                    update_sender_and_timestamp(block, endpoint)
                }))
            }
        }
    }

    /// Public method to send an outgoing block from this endpoint. Called by the runtime.
    pub async fn send_own_block_async(
        &self,
        mut block: DXBBlock,
    ) -> Result<(), Vec<Endpoint>> {
        block = self
            .prepare_own_block(block)
            .into_result()
            .await
            .unwrap_or_else(|e| {
                panic!("Error preparing own block for sending: {:?}", e)
            });

        // add own outgoing block to history
        self.block_handler
            .add_block_id_to_history(block.get_block_id(), None);
        self.send_block_async(block, vec![], false).await
    }

    /// Sends a block from this endpoint synchronously.
    /// If any endpoint can not be reached synchronously, an Err with the list of all endpoints is returned.
    /// Otherwise, Ok with optional list of responses is returned.
    pub fn send_own_block(
        &self,
        mut block: DXBBlock,
    ) -> Result<Option<Vec<Vec<u8>>>, Vec<Endpoint>> {
        let receivers = block.receiver_endpoints();
        block = match self.prepare_own_block(block) {
            SyncOrAsync::Sync(res) => res.unwrap_or_else(|e| {
                panic!("Error preparing own block for sending: {:?}", e)
            }),
            SyncOrAsync::Async(_) => {
                return Err(receivers);
            }
        };
        self.block_handler
            .add_block_id_to_history(block.get_block_id(), None);
        match self.send_block(block, vec![], false) {
            BlockSendSyncOrAsyncResult::Sync(res) => res,
            BlockSendSyncOrAsyncResult::Async(_) => Err(receivers),
        }
    }

    /// Sends a block and wait for a response block.
    /// Fix number of exact endpoints -> Expected responses are known at send time.
    /// TODO #189: make sure that mutating blocks are always send to specific endpoint instances (@jonas/0001), not generic endpoints like @jonas.
    /// @jonas -> response comes from a specific instance of @jonas/0001
    pub async fn send_own_block_await_response(
        &self,
        block: DXBBlock,
        options: ResponseOptions,
    ) -> Vec<Result<Response, ResponseError>> {
        let context_id = block.block_header.context_id;
        let section_index = block.block_header.section_index;

        let mut rx = self
            .block_handler
            .register_incoming_block_observer(context_id, section_index);

        let has_exact_receiver_count = block.has_exact_receiver_count();
        let receivers = block.receiver_endpoints();

        let res = self.send_own_block_async(block).await;
        let failed_endpoints = res.err().unwrap_or_default();

        let timeout = options
            .timeout
            .unwrap_or_default(self.options.default_receive_timeout);

        // return fixed number of responses
        if has_exact_receiver_count {
            // if resolution strategy is ReturnOnAnyError or ReturnOnFirstResult, directly return if any endpoint failed
            if (options.resolution_strategy
                == ResponseResolutionStrategy::ReturnOnAnyError
                || options.resolution_strategy
                    == ResponseResolutionStrategy::ReturnOnFirstResult)
                && !failed_endpoints.is_empty()
            {
                // for each failed endpoint, set NotReachable error, for all others EarlyAbort
                return receivers
                    .iter()
                    .map(|receiver| {
                        if failed_endpoints.contains(receiver) {
                            Err(ResponseError::NotReachable(receiver.clone()))
                        } else {
                            Err(ResponseError::EarlyAbort(receiver.clone()))
                        }
                    })
                    .collect::<Vec<_>>();
            }

            // store received responses in map for all receivers
            let mut responses = HashMap::new();
            let mut missing_response_count = receivers.len();
            for receiver in &receivers {
                responses.insert(
                    receiver.clone(),
                    if failed_endpoints.contains(receiver) {
                        Err(ResponseError::NotReachable(receiver.clone()))
                    } else {
                        Err(ResponseError::NoResponseAfterTimeout(
                            receiver.clone(),
                            timeout,
                        ))
                    },
                );
            }
            // directly subtract number of already failed endpoints from missing responses
            missing_response_count -= failed_endpoints.len();

            info!(
                "Waiting for responses from receivers {}",
                receivers
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            let res = task::timeout(timeout, async {
                while let Some(section) = rx.next().await {
                    let mut received_response = false;
                    // get sender
                    let mut sender = section.get_sender();
                    // add to response for exactly matching endpoint instance
                    if let Some(response) = responses.get_mut(&sender) {
                        // check if the receiver is already set (= current set response is Err)
                        if response.is_err() {
                            *response = Ok(Response::ExactResponse(sender.clone(), section));
                            missing_response_count -= 1;
                            info!("Received expected response from {sender}");
                            received_response = true;
                        }
                        // already received a response from this exact sender - this should not happen
                        else {
                            error!("Received multiple responses from the same sender: {sender}");
                        }
                    }
                    // add to response for matching endpoint
                    else if let Some(matches_endpoint) = self.try_match_sender(&mut responses, &sender) {
                        let response = responses.get_mut(&matches_endpoint).unwrap();
                        info!("Received resolved response from {} -> {}", sender, sender.any_instance());
                        sender = sender.any_instance();
                        // check if the receiver is already set (= current set response is Err)
                        if response.is_err() {
                            *response = Ok(Response::ResolvedResponse(sender.clone(), section));
                            missing_response_count -= 1;
                            received_response = true;
                        }
                        // already received a response from a matching endpoint - ignore
                        else {
                            info!("Received multiple resolved responses from the {}", sender);
                        }
                    }
                    // response from unexpected sender
                    else {
                        error!("Received response from unexpected sender: {}", sender);
                    }

                    // if resolution strategy is ReturnOnFirstResult, break if any response is received
                    if received_response && options.resolution_strategy == ResponseResolutionStrategy::ReturnOnFirstResult {
                        // set all other responses to EarlyAbort
                        for (receiver, response) in responses.iter_mut() {
                            if receiver != &sender {
                                *response = Err(ResponseError::EarlyAbort(receiver.clone()));
                            }
                        }
                        break;
                    }

                    // if all responses are received, break
                    if missing_response_count == 0 {
                        break;
                    }
                }
            }).await;

            if res.is_err() {
                error!("Timeout waiting for responses");
            }

            // return responses as vector
            responses.into_values().collect::<Vec<_>>()
        }
        // return all received responses
        else {
            let mut responses = vec![];

            let res = task::timeout(timeout, async {
                let mut rx =
                    self.block_handler.register_incoming_block_observer(
                        context_id,
                        section_index,
                    );
                while let Some(section) = rx.next().await {
                    // get sender
                    let sender = section.get_sender();
                    info!("Received response from {sender}");
                    // add to response for exactly matching endpoint instance
                    responses.push(Ok(Response::UnspecifiedResponse(section)));

                    // if resolution strategy is ReturnOnFirstResult, break if any response is received
                    if options.resolution_strategy
                        == ResponseResolutionStrategy::ReturnOnFirstResult
                    {
                        break;
                    }
                }
            })
            .await;

            if res.is_err() {
                info!("Timeout waiting for responses");
            }

            responses
        }
    }

    /// Tries to match the sender endpoint to a more generic endpoint in the responses map (e.g., @jonas/0001 -> @jonas or @@local/0001 -> @xyz) and returns the matching endpoint if found.
    fn try_match_sender(
        &self,
        responses: &mut HashMap<Endpoint, Result<Response, ResponseError>>,
        sender: &Endpoint,
    ) -> Option<Endpoint> {
        let matches = gen {
            // match sender but with any wildcard instance
            yield sender.any_instance();
            // match @@local if endpoint is local endpoint
            if self.is_local_endpoint_exact(sender) {
                yield Endpoint::LOCAL;
                yield Endpoint::LOCAL_ALL_INSTANCES;
            }
        }
        .collect::<Vec<Endpoint>>();
        for try_match_sender in matches {
            let res = responses.get(&try_match_sender);
            if let Some(_response) = res {
                return Some(try_match_sender);
            }
        }
        None
    }

    /// Sends a block to all endpoints specified in the block header.
    /// Awaits the result if any block was sent via an async interface.
    /// See `send_block` for details.
    pub async fn send_block_async(
        &self,
        block: DXBBlock,
        exclude_sockets: Vec<ComInterfaceSocketUUID>,
        forked: bool,
    ) -> Result<(), Vec<Endpoint>> {
        match self.send_block(block, exclude_sockets, forked) {
            SyncOrAsyncResult::Sync(res) => {
                // TODO #735: handle received blocks
                res.map(|_| ())
            }
            SyncOrAsyncResult::Async(fut) => fut.await,
        }
    }

    /// Sends a block to all endpoints specified in the block header.
    /// The routing algorithm decides which sockets are used to send the block, based on the endpoint.
    /// A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
    /// The original_socket parameter is used to prevent sending the block back to the sender.
    /// When this method is called, the block is queued in the send queue.
    /// Returns a SyncOrAsyncResult:
    ///  - if all blocks were sent via sync interfaces, returns Sync with Ok containing an optional vector of received blocks (if any), or Err with a list of unreachable endpoints
    ///  - if any block was sent via an async interface, returns Async with a Future that resolves to Ok(()) or Err with a list of unreachable endpoints
    pub fn send_block(
        &self,
        mut block: DXBBlock,
        exclude_sockets: Vec<ComInterfaceSocketUUID>,
        forked: bool,
    ) -> BlockSendSyncOrAsyncResult<
        impl Future<Output = Result<(), Vec<Endpoint>>>,
    > {
        let outbound_receiver_groups =
            self.socket_manager.get_outbound_receiver_groups(
                &self.endpoint,
                &block.receiver_endpoints(),
                exclude_sockets,
            );

        if outbound_receiver_groups.is_none() {
            error!("No outbound receiver groups found for block");
            return SyncOrAsyncResult::Sync(Err(vec![]));
        }

        let outbound_receiver_groups = outbound_receiver_groups.unwrap();

        let mut unreachable_endpoints = vec![];

        // currently only used for trace debugging (TODO: put behind debug flag)
        // if more than one addressed block is sent, the block is forked, thus the fork count is set to 0
        // for each forked block, the fork count is incremented
        // if only one block is sent, the block is just moved and not forked
        let mut fork_count = if forked || outbound_receiver_groups.len() > 1 {
            Some(0)
        } else {
            None
        };

        block.set_bounce_back(false);

        let mut results = Vec::new();

        for (receiver_socket, endpoints) in outbound_receiver_groups {
            if let Some(socket_uuid) = receiver_socket {
                results.push((
                    endpoints.clone(),
                    self.send_block_to_endpoints_via_socket(
                        block.clone(),
                        socket_uuid,
                        endpoints,
                        fork_count,
                    ),
                ));
            } else {
                error!(
                    "{}: cannot send block, no receiver sockets found for endpoints {:?}",
                    self.endpoint,
                    endpoints.iter().map(|e| e.to_string()).collect::<Vec<_>>()
                );
                unreachable_endpoints.extend(endpoints);
            }
            // increment fork_count if Some
            if let Some(count) = fork_count {
                fork_count = Some(count + 1);
            }
        }

        // return error if any unreachable endpoints
        if !unreachable_endpoints.is_empty() {
            return SyncOrAsyncResult::Sync(Err(unreachable_endpoints));
        }

        // if all results are sync, return sync
        if results
            .iter()
            .all(|(_, res)| matches!(res, SyncOrAsync::Sync(_)))
        {
            let mut received_blocks = Vec::new();
            for (endpoints, res) in results {
                match res {
                    SyncOrAsync::Sync(r) => {
                        match r {
                            Ok(Some(data)) => {
                                received_blocks.push(data); // TODO #736: already DXBBlocks here?
                            }
                            Ok(None) => { /* no data */ }
                            Err(_) => {
                                unreachable_endpoints.extend(endpoints);
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            if !unreachable_endpoints.is_empty() {
                SyncOrAsyncResult::Sync(Err(unreachable_endpoints))
            } else {
                SyncOrAsyncResult::Sync(Ok(Some(received_blocks)))
            }
        }
        // otherwise return async
        else {
            SyncOrAsyncResult::Async(async move {
                let futures =
                    results.into_iter().map(|(endpoints, res)| async move {
                        match res {
                            SyncOrAsync::Sync(r) => {
                                // TODO #737 directly process received blocks
                                r.map(|_data| ()).map_err(|_| endpoints)
                            }
                            SyncOrAsync::Async(fut) => {
                                fut.await.map_err(|_| endpoints)
                            }
                        }
                    });

                let res = futures::future::join_all(futures).await;
                // merge all unreachable endpoints and return err if any
                let all_unreachable_endpoints = res
                    .into_iter()
                    .filter_map(|r| r.err())
                    .flatten()
                    .collect::<Vec<_>>();
                if !all_unreachable_endpoints.is_empty() {
                    Err(all_unreachable_endpoints)
                } else {
                    Ok(())
                }
            })
        }
    }

    /// Sends a block via a socket to a list of endpoints.
    /// Before the block is sent, it is modified to include the list of endpoints as receivers.
    fn send_block_to_endpoints_via_socket(
        &self,
        mut block: DXBBlock,
        socket_uuid: ComInterfaceSocketUUID,
        endpoints: Vec<Endpoint>,
        // currently only used for trace debugging (TODO: put behind debug flag)
        fork_count: Option<usize>,
    ) -> SyncOrAsyncResult<
        Option<Vec<u8>>,
        (),
        SendFailure,
        impl Future<Output = Result<(), SendFailure>>,
    > {
        block.set_receivers(&endpoints);

        let socket_data =
            match self.socket_manager.get_socket_by_uuid(&socket_uuid) {
                Some(socket_data) => socket_data,
                None => {
                    return SyncOrAsyncResult::Sync(Err(SendFailure(
                        Box::new(block),
                    )));
                }
            };

        // assuming the distance was already increment during redirect, we
        // effectively decrement the block distance by 1 if it is a bounce back
        if block.is_bounce_back() {
            block.routing_header.distance -= 2;
        }

        // if type is Trace or TraceBack, add the outgoing socket to the hops
        // NOTE: the block body is modified here for trace blocks, which would invalidate any signature
        // generated for the block. However, trace blocks are only used for debugging purposes and not expected to be signed.
        match block.block_header.flags_and_timestamp.block_type() {
            BlockType::Trace | BlockType::TraceBack => {
                let distance = block.routing_header.distance;
                let new_fork_nr = self.calculate_fork_nr(&block, fork_count);
                let bounce_back = block.is_bounce_back();

                self.add_hop_to_block_trace_data(
                    &mut block,
                    NetworkTraceHop {
                        endpoint: self.endpoint.clone(),
                        distance,
                        socket: NetworkTraceHopSocket::new(
                            &socket_data.interface_properties,
                            socket_uuid.clone(),
                        ),
                        direction: NetworkTraceHopDirection::Outgoing,
                        fork_nr: new_fork_nr,
                        bounce_back,
                    },
                );
            }
            _ => {}
        }

        let is_broadcast = endpoints
            .iter()
            .any(|e| e == &Endpoint::ANY_ALL_INSTANCES || e == &Endpoint::ANY);

        // Break loop and don't relay broadcast blocks back to socket with direct endpoint set to self
        if is_broadcast
            && let Some(direct_endpoint) =
                &socket_data.socket_properties.direct_endpoint
            && self.is_local_endpoint_exact(direct_endpoint)
        {
            return SyncOrAsyncResult::Sync(Ok(None));
        }
        for interceptor in self.outgoing_block_interceptors.borrow().iter() {
            interceptor(&block, &socket_uuid, &endpoints);
        }
        info!(
            "Sending block to socket {}: {}",
            socket_uuid,
            endpoints.iter().map(|e| e.to_string()).join(", ")
        );

        // TODO #190: resend block if socket failed to send
        if let Some(send_callback) = socket_data.send_callback.clone() {
            match send_callback {
                SendCallback::Sync(callback)
                | SendCallback::SyncOnce(callback) => SyncOrAsyncResult::Sync(
                    callback(block).map(|send_success| match send_success {
                        SendSuccess::SentWithNewIncomingData(data) => {
                            Some(data)
                        }
                        _ => None,
                    }),
                ),
                SendCallback::Async(callback) => {
                    SyncOrAsyncResult::Async(async move {
                        callback.call(block).await.map(|_| ())
                    })
                }
            }
        } else {
            panic!("No send callback registered for socket {}", socket_uuid);
        }
    }

    /// Sends a hello block via the specified socket.
    /// Returns Ok(()) if the block was sent successfully, or Err(SendFailure) if sending failed.
    pub async fn send_hello_block(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Result<(), SendFailure> {
        let mut block: DXBBlock = DXBBlock::default();
        block
            .block_header
            .flags_and_timestamp
            .set_block_type(BlockType::Hello);
        // TODO #182 include fingerprint of the own public key into body

        let block = self
            .prepare_own_block(block)
            .into_result()
            .await
            .unwrap_or_else(|e| {
                panic!("Error preparing own block for sending: {:?}", e)
            });

        match self
            .send_block_to_endpoints_via_socket(
                block,
                socket_uuid.clone(),
                vec![Endpoint::ANY],
                None,
            )
            .into_future()
            .await
        {
            SyncOrAsyncResolved::Sync(r) => r.map(|_| ()),
            SyncOrAsyncResolved::Async(fut) => fut,
        }
    }

    pub fn clear_endpoint_blacklist(&self) {
        self.socket_manager
            .endpoint_sockets_blacklist
            .borrow_mut()
            .clear();
    }

    pub fn interfaces_manager(&self) -> &ComInterfaceManager {
        &self.interfaces_manager
    }

    pub fn socket_manager(&self) -> &ComInterfaceSocketManager {
        &self.socket_manager
    }
}

#[cfg(test)]
#[cfg(feature = "crypto_enabled")]
pub mod tests {
    use crate::{
        channel::mpsc::{
            UnboundedReceiver, UnboundedSender, create_unbounded_channel,
        },
        global::{
            dxb_block::{DXBBlock, IncomingSection},
            protocol_structures::{
                block_header::{BlockHeader, FlagsAndTimestamp},
                routing_header::RoutingHeader,
            },
        },
        network::{
            com_hub::{
                ComHub, InterfacePriority,
                errors::ComInterfaceCreateError,
                managers::com_interface_manager::{
                    ComInterfaceAsyncFactoryResult, ComInterfaceManager,
                },
                test_utils::{
                    TEST_ENDPOINT_A, TEST_ENDPOINT_B, TEST_ENDPOINT_C,
                    get_endpoints_from_com_hub_metadata,
                    run_with_coupled_com_hubs,
                },
            },
            com_interfaces::com_interface::{
                factory::{
                    ComInterfaceAsyncFactory, ComInterfaceConfiguration,
                    ComInterfaceSyncFactory, SendCallback, SendSuccess,
                    SocketConfiguration, SocketProperties,
                },
                properties::{ComInterfaceProperties, InterfaceDirection},
            },
        },
        prelude::*,
        std_sync::Mutex,
        values::core_values::endpoint::Endpoint,
    };
    use alloc::rc::Rc;
    use async_select::select;
    use datex_macros_internal::Datex;
    use futures_util::FutureExt;
    use log::info;
    use serde::Deserialize;
    use tokio::task::yield_now;

    /// Creates a mock ComHub for testing without a connected channel
    async fn run_with_com_hub<AppReturn, AppFuture>(
        app_logic: impl FnOnce(
            Rc<ComHub>,
            UnboundedReceiver<IncomingSection>,
        ) -> AppFuture,
    ) -> AppReturn
    where
        AppFuture: Future<Output = AppReturn>,
    {
        let (sender, receiver) = create_unbounded_channel();
        let (com_hub, com_hub_future) =
            ComHub::create(TEST_ENDPOINT_A.clone(), sender);
        select! {
            app_result = app_logic(com_hub, receiver).fuse() => app_result,
            _ = com_hub_future.fuse() => panic!("ComHub future should not complete during the test"),
        }
    }

    async fn add_proxy_interface_to_com_hub(
        com_hub: Rc<ComHub>,
        endpoint: Endpoint,
    ) -> (UnboundedSender<Vec<u8>>, UnboundedReceiver<DXBBlock>) {
        let (outgoing_block_sender, outgoing_block_receiver) =
            create_unbounded_channel();
        let (incoming_data_sender, mut incoming_data_receiver) =
            create_unbounded_channel::<Vec<u8>>();
        let outgoing_block_sender = Rc::new(Mutex::new(outgoing_block_sender));

        let proxy_interface_configuration =
            ComInterfaceConfiguration::new_single_socket(
                ComInterfaceProperties {
                    interface_type: "proxy".to_string(),
                    channel: "proxy".to_string(),
                    name: Some("proxy".to_string()),
                    ..Default::default()
                },
                SocketConfiguration::new_in_out(
                    SocketProperties::new_with_direct_endpoint(
                        InterfaceDirection::InOut,
                        1,
                        endpoint,
                    ),
                    async gen move {
                        while let Some(block) =
                            incoming_data_receiver.next().await
                        {
                            yield Ok(block)
                        }
                    },
                    SendCallback::new_sync(move |block| {
                        outgoing_block_sender
                            .try_lock()
                            .unwrap()
                            .start_send(block)
                            .unwrap();
                        Ok(SendSuccess::Sent)
                    }),
                ),
            );
        com_hub
            .clone()
            .add_interface_from_configuration(
                proxy_interface_configuration,
                InterfacePriority::None,
            )
            .unwrap()
            .unwrap()
            .await
            .unwrap();

        (incoming_data_sender, outgoing_block_receiver)
    }

    async fn run_with_com_hub_and_proxy_interface<AppReturn, AppFuture>(
        app_logic: impl FnOnce(
            Rc<ComHub>,
            UnboundedSender<Vec<u8>>,
            UnboundedReceiver<DXBBlock>,
            UnboundedReceiver<IncomingSection>,
        ) -> AppFuture,
    ) -> AppReturn
    where
        AppFuture: Future<Output = AppReturn>,
    {
        run_with_com_hub(|com_hub, incoming_sections_receiver| async move {
            let (incoming_data_sender, outgoing_block_receiver) =
                add_proxy_interface_to_com_hub(
                    com_hub.clone(),
                    TEST_ENDPOINT_B.clone(),
                )
                .await;
            app_logic(
                com_hub,
                incoming_data_sender,
                outgoing_block_receiver,
                incoming_sections_receiver,
            )
            .await
        })
        .await
    }

    async fn send_blocks_to_endpoint(
        com_hub: Rc<ComHub>,
        incoming_data_sender: &mut UnboundedSender<Vec<u8>>,
        endpoint: Endpoint,
        blocks: &mut Vec<DXBBlock>,
    ) {
        for mut block in blocks {
            block.set_receivers(vec![endpoint.clone()]);

            *block = com_hub
                .prepare_own_block(block.clone())
                .into_result()
                .await
                .unwrap();

            let block_bytes = block.to_bytes();
            incoming_data_sender
                .start_send(block_bytes.as_slice().to_vec())
                .unwrap();
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    pub enum CollectedBlockType {
        #[default]
        All,
        SingleBocks,
        BlockStream,
    }

    impl CollectedBlockType {
        pub fn matches_section(&self, section: &IncomingSection) -> bool {
            match self {
                CollectedBlockType::SingleBocks => {
                    matches!(section, IncomingSection::SingleBlock(_))
                }
                CollectedBlockType::BlockStream => {
                    matches!(section, IncomingSection::BlockStream(_))
                }
                CollectedBlockType::All => true,
            }
        }
    }

    pub async fn get_collected_received_blocks_from_receiver(
        sections_receiver: &mut UnboundedReceiver<IncomingSection>,
        collected_type: CollectedBlockType,
        count: usize,
    ) -> Vec<DXBBlock> {
        let mut blocks = vec![];

        let mut received_count = 0;

        while let Some(section) = sections_receiver.next().await {
            if !collected_type.matches_section(&section) {
                panic!(
                    "Received section does not match collected block type {:?}",
                    collected_type
                );
            }

            match section {
                IncomingSection::SingleBlock((Some(block), ..)) => {
                    blocks.push(block.clone());
                    received_count += 1;
                    info!("Received single block");
                }
                IncomingSection::BlockStream((Some(mut block_stream), ..)) => {
                    info!("[START] block stream");
                    while let Some(block) = block_stream.next().await {
                        received_count += 1;
                        blocks.push(block.clone());
                        info!("Received block from stream");

                        if received_count >= count {
                            break;
                        }
                    }
                    info!("[END] receiving block stream");
                }
                _ => {
                    panic!("Received section does not contain a block");
                }
            }

            if received_count >= count {
                break;
            }
        }

        if blocks.len() != count {
            panic!(
                "Expected to receive {} blocks, but got {}",
                count,
                blocks.len()
            );
        }

        blocks
    }

    #[derive(Datex, Debug, Clone)]
    #[datex(structural)]
    struct MockupInterfaceSetupData {
        pub name: String,
    }
    impl MockupInterfaceSetupData {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    impl ComInterfaceSyncFactory for MockupInterfaceSetupData {
        fn create_interface(
            self,
        ) -> Result<ComInterfaceConfiguration, ComInterfaceCreateError>
        {
            Ok(ComInterfaceConfiguration::new_single_socket(
                ComInterfaceProperties::default(),
                SocketConfiguration::new_in_out(
                    SocketProperties::new(InterfaceDirection::InOut, 1),
                    async gen move {
                        loop {
                            yield Ok(vec![]);
                        }
                    },
                    SendCallback::new_sync(|_| Ok(SendSuccess::Sent)),
                ),
            ))
        }

        fn get_default_properties() -> ComInterfaceProperties {
            ComInterfaceProperties {
                name: Some("mockup".to_string()),
                ..Default::default()
            }
        }
    }

    impl ComInterfaceAsyncFactory for MockupInterfaceSetupData {
        fn create_interface(self) -> ComInterfaceAsyncFactoryResult {
            Box::pin(
                async move { ComInterfaceSyncFactory::create_interface(self) },
            )
        }
        fn get_default_properties() -> ComInterfaceProperties {
            <MockupInterfaceSetupData as ComInterfaceSyncFactory>::get_default_properties()
        }
    }

    #[tokio::test]
    pub async fn create_from_sync_factory() {
        run_with_com_hub(|com_hub, _| async move {
            let interface_configuration =
                ComInterfaceManager::create_interface_sync_from_setup_data(
                    MockupInterfaceSetupData::new("test"),
                )
                .unwrap();
            let uuid = interface_configuration.uuid().clone();

            com_hub
                .clone()
                .add_interface_from_configuration(
                    interface_configuration,
                    InterfacePriority::default(),
                )
                .unwrap();

            assert!(com_hub.remove_interface(uuid).await.is_ok());
        })
        .await;
    }

    #[tokio::test]
    pub async fn create_from_async_factory() {
        run_with_com_hub(|com_hub, _| async move {
            let interface_configuration =
                ComInterfaceManager::create_interface_async_from_setup_data(
                    MockupInterfaceSetupData::new("test"),
                )
                .await
                .unwrap();
            let uuid = interface_configuration.uuid().clone();

            com_hub
                .clone()
                .add_interface_from_configuration(
                    interface_configuration,
                    InterfacePriority::default(),
                )
                .unwrap();

            assert!(com_hub.remove_interface(uuid).await.is_ok());
        })
        .await;
    }

    #[tokio::test]
    async fn create_hello_connection() {
        run_with_coupled_com_hubs(async |a, b| {
            let com_hub_a_sockets =
                get_endpoints_from_com_hub_metadata(a.com_hub.get_metadata());
            assert!(
                com_hub_a_sockets
                    .contains(&(Some(TEST_ENDPOINT_B.clone()), Some(1)))
            );

            let com_hub_b_sockets =
                get_endpoints_from_com_hub_metadata(b.com_hub.get_metadata());
            assert!(
                com_hub_b_sockets
                    .contains(&(Some(TEST_ENDPOINT_A.clone()), Some(1)))
            );
        })
        .await;
    }

    #[tokio::test]
    pub async fn send() {
        run_with_com_hub_and_proxy_interface(
            async move |com_hub, _, mut outgoing_block_receiver, _| {
                // send block via com hub to proxy interface
                let mut block = DXBBlock::new_with_body(b"Hello world!");
                block.set_receivers(vec![TEST_ENDPOINT_B.clone()]);
                com_hub.send_own_block_async(block).await.unwrap();

                // hello block, skip
                outgoing_block_receiver.next().await.unwrap();

                // get next outgoing block that was sent via the proxy interface
                let outgoing_block =
                    outgoing_block_receiver.next().await.unwrap();
                assert_eq!(outgoing_block.body, b"Hello world!");
            },
        )
        .await;
    }

    #[tokio::test]
    pub async fn send_between_com_hubs() {
        run_with_coupled_com_hubs(async |a, mut b| {
            // send block via com hub to proxy interface
            let mut block = DXBBlock::new_with_body(b"Hello world!");
            block.set_receivers(vec![TEST_ENDPOINT_B.clone()]);
            a.com_hub.send_own_block_async(block).await.unwrap();

            // get received single block on com hub b
            let next_block = b.incoming_sections_receiver.next().await.unwrap();
            if let IncomingSection::SingleBlock((Some(block), _)) = next_block {
                assert_eq!(block.body, b"Hello world!");
            } else {
                panic!("Expected single block section");
            }
        })
        .await;
    }

    #[tokio::test]
    pub async fn send_block_to_invalid_receiver() {
        run_with_com_hub_and_proxy_interface(async move |com_hub, _, _, _| {
            let mut block = DXBBlock::new_with_body(b"Hello world!");
            // cannot send to endpoint c, since only interface is not a fallback interface and only knows endpoint b
            block.set_receivers(vec![TEST_ENDPOINT_C.clone()]);
            let res = com_hub.send_own_block_async(block).await;
            assert!(res.is_err());
        })
        .await;
    }

    #[tokio::test]
    pub async fn send_block_via_multiple_interfaces() {
        run_with_com_hub(|com_hub, _| async move {
            let (_sender_b, mut outgoing_block_receiver_b) =
                add_proxy_interface_to_com_hub(
                    com_hub.clone(),
                    TEST_ENDPOINT_B.clone(),
                )
                .await;
            let (_sender_c, mut outgoing_block_receiver_c) =
                add_proxy_interface_to_com_hub(
                    com_hub.clone(),
                    TEST_ENDPOINT_C.clone(),
                )
                .await;
            com_hub.print_metadata();

            let mut block = DXBBlock::new_with_body(b"Hello world!");
            block.set_receivers(vec![
                TEST_ENDPOINT_B.clone(),
                TEST_ENDPOINT_C.clone(),
            ]);
            com_hub.send_own_block_async(block).await.unwrap();

            // skip hello blocks
            outgoing_block_receiver_b.next().await.unwrap();
            outgoing_block_receiver_c.next().await.unwrap();

            // block should be sent via both interfaces
            let outgoing_block_b =
                outgoing_block_receiver_b.next().await.unwrap();
            let outgoing_block_c =
                outgoing_block_receiver_c.next().await.unwrap();

            info!("block sender b: {}", outgoing_block_b.sender());
            info!("block sender c: {}", outgoing_block_c.sender());
            assert_eq!(outgoing_block_b.body, b"Hello world!");
            assert_eq!(outgoing_block_c.body, b"Hello world!");
        })
        .await
    }

    #[tokio::test]
    pub async fn receive() {
        flexi_logger::init();
        run_with_com_hub_and_proxy_interface(
            async move |com_hub,
                        mut incoming_data_sender,
                        _,
                        mut incoming_sections_receiver| {
                // create block and send it via the incoming data sender to the com hub
                let mut block = DXBBlock::new_with_body(b"Hello world!");
                block.set_receivers(vec![TEST_ENDPOINT_A.clone()]);

                let block = com_hub
                    .prepare_own_block(block)
                    .into_result()
                    .await
                    .unwrap();

                let block_bytes = block.to_bytes();
                incoming_data_sender
                    .start_send(block_bytes.as_slice().to_vec())
                    .unwrap();

                let incoming_section =
                    incoming_sections_receiver.next().await.unwrap();
                if let IncomingSection::SingleBlock((Some(block), _)) =
                    incoming_section
                {
                    assert_eq!(block.raw_bytes.clone().unwrap(), block_bytes);
                } else {
                    panic!("Expected single block section");
                }
            },
        )
        .await;
    }

    #[tokio::test]
    pub async fn receive_multiple_blocks_single_section() {
        run_with_com_hub_and_proxy_interface(
            async move |com_hub,
                        mut incoming_data_sender,
                        _,
                        mut incoming_sections_receiver| {
                let mut blocks = vec![
                    DXBBlock {
                        routing_header: RoutingHeader::default(),
                        block_header: BlockHeader {
                            section_index: 0,
                            block_number: 0,
                            flags_and_timestamp: FlagsAndTimestamp::new()
                                .with_is_end_of_section(false)
                                .with_is_end_of_context(false),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    DXBBlock {
                        routing_header: RoutingHeader::default(),
                        block_header: BlockHeader {
                            section_index: 0,
                            block_number: 1,
                            flags_and_timestamp: FlagsAndTimestamp::new()
                                .with_is_end_of_section(false)
                                .with_is_end_of_context(false),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    DXBBlock {
                        routing_header: RoutingHeader::default(),
                        block_header: BlockHeader {
                            section_index: 0,
                            block_number: 2,
                            flags_and_timestamp: FlagsAndTimestamp::new()
                                .with_is_end_of_section(true)
                                .with_is_end_of_context(true),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ];
                let blocks_count = blocks.len();

                send_blocks_to_endpoint(
                    com_hub.clone(),
                    &mut incoming_data_sender,
                    TEST_ENDPOINT_A.clone(),
                    &mut blocks,
                )
                .await;

                let incoming_blocks =
                    get_collected_received_blocks_from_receiver(
                        &mut incoming_sections_receiver,
                        CollectedBlockType::BlockStream,
                        blocks_count,
                    )
                    .await;

                for (incoming_block, block) in
                    incoming_blocks.iter().zip(blocks.iter())
                {
                    assert_eq!(
                        incoming_block.raw_bytes.clone().unwrap(),
                        block.to_bytes()
                    );
                }
            },
        )
        .await;
    }

    #[tokio::test]
    pub async fn receive_multiple_separate_blocks() {
        run_with_com_hub_and_proxy_interface(
            async move |com_hub,
                        mut incoming_data_sender,
                        _,
                        mut incoming_sections_receiver| {
                let mut blocks = vec![
                    DXBBlock {
                        routing_header: RoutingHeader::default(),
                        block_header: BlockHeader {
                            section_index: 1,
                            block_number: 0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    DXBBlock {
                        routing_header: RoutingHeader::default(),
                        block_header: BlockHeader {
                            section_index: 2,
                            block_number: 0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    DXBBlock {
                        routing_header: RoutingHeader::default(),
                        block_header: BlockHeader {
                            section_index: 3,
                            block_number: 0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ];
                let blocks_count = blocks.len();

                send_blocks_to_endpoint(
                    com_hub.clone(),
                    &mut incoming_data_sender,
                    TEST_ENDPOINT_A.clone(),
                    &mut blocks,
                )
                .await;

                let incoming_blocks =
                    get_collected_received_blocks_from_receiver(
                        &mut incoming_sections_receiver,
                        CollectedBlockType::SingleBocks,
                        blocks_count,
                    )
                    .await;

                for (incoming_block, block) in
                    incoming_blocks.iter().zip(blocks.iter())
                {
                    assert_eq!(
                        incoming_block.raw_bytes.clone().unwrap(),
                        block.to_bytes()
                    );
                }
            },
        )
        .await;
    }

    // #[async_test]
    // pub async fn unencrypted_signature_prepare_block_com_hub() {
    //     let (com_hub, interface_proxy, mut incoming_sections_receiver) =
    //         get_default_mock_setup_with_com_hub().await;
    //
    //     // receive block
    //     let mut block = DXBBlock {
    //         body: vec![0x01, 0x02, 0x03],
    //         encrypted_header: EncryptedHeader {
    //             flags: encrypted_header::Flags::new()
    //                 .with_user_agent(encrypted_header::UserAgent::Unused11),
    //             ..Default::default()
    //         },
    //         ..DXBBlock::default()
    //     };
    //     block.set_receivers(vec![TEST_ENDPOINT_ORIGIN.clone()]);
    //     block.recalculate_struct();
    //     block
    //         .routing_header
    //         .flags
    //         .set_signature_type(SignatureType::Unencrypted);
    //     block = com_hub.prepare_own_block(block).await.unwrap();
    //     let block_bytes = block.to_bytes();
    //
    //     let (_, mut incoming_blocks_sender) =
    //         interface_proxy.create_and_init_socket(InterfaceDirection::In, 0);
    //     yield_now().await;
    //
    //     incoming_blocks_sender
    //         .start_send(block_bytes.as_slice().to_vec())
    //         .unwrap();
    //
    //     yield_now().await;
    //
    //     let last_block = get_next_received_single_block_from_receiver(
    //         &mut incoming_sections_receiver,
    //     )
    //     .await;
    //     assert_eq!(last_block.raw_bytes.clone().unwrap(), block_bytes);
    //     assert_eq!(block.signature, last_block.signature);
    //
    //     assert!(com_hub.validate_block(&last_block).await.unwrap());
    // }
    //
    // #[async_test]
    // pub async fn encrypted_signature_prepare_block_com_hub() {
    //     let (com_hub, interface_proxy, mut incoming_sections_receiver) =
    //         get_default_mock_setup_with_com_hub().await;
    //
    //     // receive block
    //     let mut block = DXBBlock {
    //         body: vec![0x01, 0x02, 0x03],
    //         encrypted_header: EncryptedHeader {
    //             flags: encrypted_header::Flags::new()
    //                 .with_user_agent(encrypted_header::UserAgent::Unused11),
    //             ..Default::default()
    //         },
    //         ..DXBBlock::default()
    //     };
    //
    //     block.set_receivers(vec![TEST_ENDPOINT_ORIGIN.clone()]);
    //     block.recalculate_struct();
    //
    //     block
    //         .routing_header
    //         .flags
    //         .set_signature_type(SignatureType::Encrypted);
    //     block = com_hub.prepare_own_block(block).await.unwrap();
    //     let block_bytes = block.to_bytes();
    //
    //     let (_, mut incoming_blocks_sender) =
    //         interface_proxy.create_and_init_socket(InterfaceDirection::In, 0);
    //     yield_now().await;
    //
    //     incoming_blocks_sender
    //         .start_send(block_bytes.as_slice().to_vec())
    //         .unwrap();
    //
    //     yield_now().await;
    //
    //     let last_block = get_next_received_single_block_from_receiver(
    //         &mut incoming_sections_receiver,
    //     )
    //     .await;
    //     assert_eq!(last_block.raw_bytes.clone().unwrap(), block_bytes);
    //     assert_eq!(block.signature, last_block.signature);
    //
    //     assert!(com_hub.validate_block(&last_block).await.unwrap());
    // }
    //
    //
    //
    // #[async_test]
    // pub async fn reconnect() {
    //     let com_hub = create_mock_com_hub();
    //
    //     // TODO #738: refactor using proxy
    //
    //     // create a new interface, open it and add it to the com_hub
    //     let (base_interface, interface_with_receivers) =
    //         ComInterfaceProxy::create_interface(ComInterfaceProperties::default());
    //
    //     // add base_interface to com_hub
    //     com_hub
    //         ._register_com_interface(
    //             interface_with_receivers,
    //             InterfacePriority::default(),
    //         )
    //         .unwrap();
    //
    //     // check that the interface is connected
    //     assert_eq!(
    //         base_interface.state.lock().unwrap().get(),
    //         ComInterfaceState::Connected
    //     );
    //
    //     // check that the interface is in the com_hub
    //     assert_eq!(com_hub.interface_manager().borrow().interfaces.len(), 2); // loopback + base_interface
    //     assert!(com_hub.has_interface(&base_interface.uuid));
    //
    //     // simulate a disconnection by closing the interface
    //     // This action is normally done by the interface itself
    //     // but we do it manually here to test the reconnection
    //     // TODO #739: reconnect
    //     // // check that the interface is not connected
    //     // // and that the close_timestamp is set
    //     // assert_eq!(
    //     //     base_interface.state.lock().unwrap().get(),
    //     //     ComInterfaceState::NotConnected
    //     // );
    //     //
    //     // assert!(
    //     //     base_interface
    //     //         .com_interface
    //     //         .properties()
    //     //         .close_timestamp
    //     //         .is_some()
    //     // );
    //     //
    //     // // the interface should not be reconnected yet
    //     // yield_now().await;
    //     //
    //     // assert_eq!(
    //     //     base_interface.com_interface.current_state(),
    //     //     ComInterfaceState::NotConnected
    //     // );
    //     //
    //     // // wait for the reconnection to happen
    //     // tokio::time::sleep(Duration::from_secs(1)).await;
    //     //
    //     // // check that the interface is connected again
    //     // // and that the close_timestamp is reset
    //     // yield_now().await;
    //     //
    //     // assert_eq!(
    //     //     base_interface.com_interface.current_state(),
    //     //     ComInterfaceState::Connected
    //     // );
    // }
}
