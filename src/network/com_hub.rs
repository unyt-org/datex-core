use crate::global::protocol_structures::block_header::BlockType;
use crate::global::protocol_structures::routing_header::SignatureType;
use crate::network::socket_manager::{EndpointIterateOptions, SocketManager};
use crate::stdlib::boxed::Box;
use crate::stdlib::{cell::RefCell, rc::Rc};
use crate::task::{self, UnboundedReceiver, sleep, spawn_with_panic_notify};
use crate::utils::time::Time;
use core::prelude::rust_2024::*;
use core::result::Result;
use futures::channel::oneshot::Sender;
use itertools::Itertools;
use log::{debug, error, info, warn};
use crate::task::spawn_local;
use webrtc::util::vnet::interface;
use core::cmp::PartialEq;
use crate::collections::{HashMap, HashSet};
use core::fmt::{Debug, Display, Formatter};
use crate::stdlib::sync::{Arc};
use crate::std_sync::Mutex;
use core::time::Duration;
#[cfg(feature = "tokio_runtime")]
use tokio::task::yield_now;
use crate::stdlib::vec::Vec;
use crate::stdlib::vec;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use super::com_interfaces::com_interface::{
    self, ComInterfaceError, ComInterfaceState
};
use super::com_interfaces::{
    com_interface::ComInterface, com_interface_socket::ComInterfaceSocket,
};
use crate::values::core_values::endpoint::{Endpoint, EndpointInstance};
use crate::global::dxb_block::{DXBBlock, IncomingSection};
use crate::network::block_handler::{BlockHandler, BlockHistoryData};
use crate::network::com_hub_network_tracing::{NetworkTraceHop, NetworkTraceHopDirection, NetworkTraceHopSocket};
use crate::network::com_interfaces::com_interface::{ComInterfaceSocketEvent, ComInterfaceUUID};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, ReconnectionConfig,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::default_com_interfaces::local_loopback_interface::LocalLoopbackInterface;
use crate::runtime::AsyncContext;
use crate::values::value_container::ValueContainer;

#[derive(Debug, Clone)]
pub struct DynamicEndpointProperties {
    pub known_since: u64,
    pub distance: i8,
    pub is_direct: bool,
    pub channel_factor: u32,
    pub direction: InterfaceDirection,
}

pub type ComInterfaceFactoryFn =
    fn(
        setup_data: ValueContainer,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComInterfaceError>;

#[derive(Debug)]
pub struct ComHubOptions {
    default_receive_timeout: Duration,
}

impl Default for ComHubOptions {
    fn default() -> Self {
        ComHubOptions {
            default_receive_timeout: Duration::from_secs(5),
        }
    }
}

type InterfaceMap = HashMap<
    ComInterfaceUUID,
    (Rc<RefCell<dyn ComInterface>>, InterfacePriority),
>;

pub type IncomingBlockInterceptor =
    Box<dyn Fn(&DXBBlock, &ComInterfaceSocketUUID) + 'static>;

pub type OutgoingBlockInterceptor =
    Box<dyn Fn(&DXBBlock, &ComInterfaceSocketUUID, &[Endpoint]) + 'static>;

pub struct ComHub {
    /// the runtime endpoint of the hub (@me)
    pub endpoint: Endpoint,

    pub async_context: AsyncContext,

    /// ComHub configuration options
    pub options: ComHubOptions,

    /// a list of all available interface factories, keyed by their interface type
    pub interface_factories: RefCell<HashMap<String, ComInterfaceFactoryFn>>,

    /// a list of all available interfaces, keyed by their UUID
    pub interfaces: RefCell<InterfaceMap>,

    pub socket_manager: Rc<RefCell<SocketManager>>,

    /// set to true if the update loop should be running
    /// when set to false, the update loop will stop
    update_loop_running: RefCell<bool>,
    update_loop_stop_sender: RefCell<Option<Sender<()>>>,

    pub block_handler: BlockHandler,

    incoming_block_interceptors: RefCell<Vec<IncomingBlockInterceptor>>,
    outgoing_block_interceptors: RefCell<Vec<OutgoingBlockInterceptor>>,
}

impl Debug for ComHub {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ComHub")
            .field("endpoint", &self.endpoint)
            .field("options", &self.options)
            .finish()
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

#[derive(Debug, Clone, PartialEq)]
pub enum ComHubError {
    InterfaceError(ComInterfaceError),
    InterfaceOpenFailed,
    InterfaceCloseFailed,
    InterfaceAlreadyExists,
    InterfaceDoesNotExist,
    InterfaceNotConnected,
    InterfaceTypeDoesNotExist,
    InvalidInterfaceDirectionForFallbackInterface,
    NoResponse,
    SignatureError,
}

impl Display for ComHubError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ComHubError::InterfaceError(_msg) => {
                core::write!(f, "ComHubError: ComInterfaceError")
            }
            ComHubError::InterfaceCloseFailed => {
                core::write!(f, "ComHubError: Failed to close interface")
            }
            ComHubError::InterfaceNotConnected => {
                core::write!(f, "ComHubError: Interface not connected")
            }
            ComHubError::InterfaceDoesNotExist => {
                core::write!(f, "ComHubError: Interface does not exit")
            }
            ComHubError::InterfaceAlreadyExists => {
                core::write!(f, "ComHubError: Interface already exists")
            }
            ComHubError::InterfaceTypeDoesNotExist => {
                core::write!(f, "ComHubError: Type of interface does not exist")
            }
            ComHubError::InvalidInterfaceDirectionForFallbackInterface => {
                core::write!(
                    f,
                    "ComHubError: Invalid direction for fallback interface"
                )
            }
            ComHubError::NoResponse => {
                core::write!(f, "ComHubError: No response")
            }
            ComHubError::InterfaceOpenFailed => {
                core::write!(f, "ComHubError: Failed to open interface")
            }
            ComHubError::SignatureError => {
                core::write!(f, "ComHubError: CryptoError")
            }
        }
    }
}

#[derive(Debug)]
pub enum SocketEndpointRegistrationError {
    SocketDisconnected,
    SocketUninitialized,
    SocketEndpointAlreadyRegistered,
}

#[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
async fn update_loop_task(self_rc: Rc<ComHub>) {
    while *self_rc.update_loop_running.borrow() {
        self_rc.update().await;
        sleep(Duration::from_millis(1)).await;
    }
    if let Some(sender) = self_rc.update_loop_stop_sender.borrow_mut().take() {
        sender.send(()).expect("Failed to send stop signal");
    }
}

#[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
async fn reconnect_interface_task(interface_rc: Rc<RefCell<dyn ComInterface>>) {
    let interface = interface_rc.clone();
    let mut interface = interface.borrow_mut();

    let config = interface.get_properties_mut();
    config.close_timestamp = None;

    let current_attempts = config.reconnect_attempts.unwrap_or(0);
    config.reconnect_attempts = Some(current_attempts + 1);

    let res = interface.handle_open().await;
    if res {
        interface.set_state(ComInterfaceState::Connected);
        // config.reconnect_attempts = None;
    } else {
        interface.set_state(ComInterfaceState::NotConnected);
    }
}

impl ComHub {
    pub fn new(
        endpoint: impl Into<Endpoint>,
        async_context: AsyncContext,
    ) -> ComHub {
        ComHub {
            endpoint: endpoint.into(),
            async_context,
            options: ComHubOptions::default(),
            interface_factories: RefCell::new(HashMap::new()),
            interfaces: RefCell::new(HashMap::new()),
            block_handler: BlockHandler::new(),
            socket_manager: Rc::new(RefCell::new(SocketManager::default())),
            update_loop_running: RefCell::new(false),
            update_loop_stop_sender: RefCell::new(None),
            incoming_block_interceptors: RefCell::new(Vec::new()),
            outgoing_block_interceptors: RefCell::new(Vec::new()),
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
        setup_data: ValueContainer,
        priority: InterfacePriority,
    ) -> Result<Rc<RefCell<dyn ComInterface>>, ComHubError> {
        info!("creating interface {interface_type}");
        let interface_factories = self.interface_factories.borrow();
        if let Some(factory) = interface_factories.get(interface_type) {
            let interface =
                factory(setup_data).map_err(ComHubError::InterfaceError)?;
            drop(interface_factories);

            self.open_and_add_interface(interface.clone(), priority)
                .await
                .map(|_| interface)
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

    pub fn get_dyn_interface_by_uuid(
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
            // FIXME #240: borrow_mut across await point
            if !(interface.borrow_mut().handle_open().await) {
                return Err(ComHubError::InterfaceOpenFailed);
            }
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

        // make sure the interface can send if a priority is set
        if priority != InterfacePriority::None
            && interface.borrow_mut().get_properties().direction
                == InterfaceDirection::In
        {
            return Err(
                ComHubError::InvalidInterfaceDirectionForFallbackInterface,
            );
        }

        interfaces.insert(uuid, (interface.clone(), priority));

        // handle socket events
        self.handle_socket_events(interface);
        // handle interface events
        self.handle_interface_events(interface);
        Ok(())
    }

    fn handle_interface_events(
        &self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) {
        let interface_event_receiver =
            interface.borrow_mut().take_interface_event_receiver();
        spawn_with_panic_notify(
            &self.async_context,
            handle_com_interface_events(interface_event_receiver),
        );
    }

    fn handle_socket_events(&self, interface: Rc<RefCell<dyn ComInterface>>) {
        let socket_event_receiver =
            interface.borrow_mut().take_socket_event_receiver();
        let priority = self
            .interfaces
            .borrow()
            .get(&interface.borrow().get_uuid())
            .map(|(_, p)| *p)
            .unwrap_or(InterfacePriority::None);
        spawn_with_panic_notify(
            &self.async_context,
            handle_com_interface_socket_events(
                socket_event_receiver,
                self.socket_manager.clone(),
                priority,
            ),
        );
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
            // FIXME #176: borrow_mut should not be used here
            let mut interface = interface.borrow_mut();
            interface.handle_destroy().await;
        }

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

    pub(crate) async fn receive_block(
        &self,
        block: &DXBBlock,
        socket_uuid: ComInterfaceSocketUUID,
    ) {
        info!("{} received block: {}", self.endpoint, block);

        // ignore invalid blocks (e.g. invalid signature)
        match self.validate_block(block).await {
            Ok(true) => { /* Ignored */ }
            Ok(false) => {
                warn!("Block validation failed. Dropping block...");
                return;
            }
            Err(e) => {
                warn!("Error in block validation {e}. Dropping block...");
                return;
            }
        }

        for interceptor in self.incoming_block_interceptors.borrow().iter() {
            interceptor(block, &socket_uuid);
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

        let receivers = block.receiver_endpoints();
        if !receivers.is_empty() {
            let is_for_own = receivers.iter().any(|e| {
                e == &self.endpoint
                    || e == &Endpoint::ANY
                    || e == &Endpoint::ANY_ALL_INSTANCES
            });

            // handle blocks for own endpoint
            if is_for_own && block_type != BlockType::Hello {
                info!("Block is for this endpoint");

                match block_type {
                    BlockType::Trace => {
                        self.handle_trace_block(block, socket_uuid.clone())
                            .await;
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

            // TODO #177: handle this via TTL, not explicitly for Hello blocks
            let should_relay =
                // don't relay "Hello" blocks sent to own endpoint
                !(
                    is_for_own && block_type == BlockType::Hello
                );

            // relay the block to other endpoints
            if should_relay {
                // get all receivers that the block must be relayed to
                let remaining_receivers = if is_for_own {
                    &self.get_remote_receivers(&receivers)
                } else {
                    &receivers
                };

                // relay the block to all receivers
                if !remaining_receivers.is_empty() {
                    match block_type {
                        BlockType::Trace | BlockType::TraceBack => {
                            self.redirect_trace_block(
                                block.clone_with_new_receivers(
                                    remaining_receivers,
                                ),
                                socket_uuid.clone(),
                                is_for_own,
                            );
                        }
                        _ => {
                            self.redirect_block(
                                block.clone_with_new_receivers(
                                    remaining_receivers,
                                ),
                                socket_uuid.clone(),
                                is_for_own,
                            );
                        }
                    }
                }
            }
        }

        // add to block history
        if is_new_block {
            self.block_handler
                .add_block_to_history(block, Some(socket_uuid));
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
        let socket = self
            .socket_manager
            .borrow()
            .get_socket_by_uuid(&socket_uuid);
        let mut socket_ref = socket.try_lock().unwrap();

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
        match self.socket_manager.borrow_mut().register_socket_endpoint(
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
                core::panic!("Failed to register socket endpoint {sender}: {error:?}");
            },
            Ok(_) => { }
        }
    }

    /// Prepares a block and relays it to the given receivers.
    /// The routing distance is incremented by 1.
    pub(crate) fn redirect_block(
        &self,
        mut block: DXBBlock,
        incoming_socket: ComInterfaceSocketUUID,
        // only for debugging traces
        forked: bool,
    ) {
        let receivers = block.receiver_endpoints();

        // check if block has already passed this endpoint (-> bounced back block)
        // and add to blacklist for all receiver endpoints
        let history_block_data =
            self.block_handler.get_block_data_from_history(&block);
        if history_block_data.is_some() {
            for receiver in &receivers {
                if receiver != &self.endpoint {
                    info!(
                        "{}: Adding socket {} to blacklist for receiver {}",
                        self.endpoint, incoming_socket, receiver
                    );
                    self.socket_manager.borrow_mut().add_to_endpoint_blocklist(
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
            return;
        // else ttl must be zero
        } else {
            warn!("Block TTL expired. Dropping block...");
            return;
        }

        let mut prefer_incoming_socket_for_bounce_back = false;
        // if we are the original sender of the block, don't send again (prevent loop) and send
        // bounce back block with all receivers
        let res = {
            if block.routing_header.sender == self.endpoint {
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
                self.send_block(block.clone(), excluded_sockets, forked)
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
                } else if self
                    .socket_manager
                    .borrow_mut()
                    .get_socket_by_uuid(&send_back_socket)
                    .try_lock()
                    .unwrap()
                    .can_send()
                {
                    block.set_bounce_back(true);
                    self.send_block_to_endpoints_via_socket(
                        block,
                        &send_back_socket,
                        &unreachable_endpoints,
                        if forked { Some(0) } else { None },
                    )
                } else {
                    error!(
                        "Tried to send bounce back block, but cannot send back to incoming socket"
                    )
                }
            }
            // Otherwise, the block originated from this endpoint, we can just call send again
            // and try to send it via other remaining sockets that are not on the blacklist for the
            // block receiver
            else {
                self.send_block(block, vec![], forked).unwrap_or_else(|_| {
                    error!(
                        "Failed to send out block to {}",
                        unreachable_endpoints
                            .iter()
                            .map(|e| e.to_string())
                            .join(",")
                    );
                });
            }
        }
    }

    /// Validates a block including it's signature if set
    /// TODO #378 @Norbert
    pub async fn validate_block(
        &self,
        block: &DXBBlock,
    ) -> Result<bool, ComHubError> {
        // TODO #179 check for creation time, withdraw if too old (TBD) or in the future

        let is_signed =
            block.routing_header.flags.signature_type() != SignatureType::None;

        match is_signed {
            true => {
                // TODO #180: verify signature and abort if invalid
                // Check if signature is following in some later block and add them to
                // a pool of incoming blocks awaiting some signature
                cfg_if::cfg_if! {
                    if #[cfg(feature = "native_crypto")] {
                        use crate::runtime::global_context::get_global_context;
                        match block.routing_header.flags.signature_type() {
                            SignatureType::Encrypted => {
                                let crypto = get_global_context().crypto;
                                let raw_sign = block.signature
                                    .as_ref()
                                    .ok_or(ComHubError::SignatureError)?;
                                let (enc_sign, pub_key) = raw_sign.split_at(64);
                                let hash = crypto.hkdf_sha256(pub_key, &[0u8; 16])
                                    .await
                                    .map_err(|_| ComHubError::SignatureError)?;
                                let signature = crypto
                                    .aes_ctr_decrypt(&hash, &[0u8; 16], enc_sign)
                                    .await
                                    .map_err(|_| ComHubError::SignatureError)?;

                                let raw_signed = [
                                    pub_key,
                                    &block.body.clone()
                                    ]
                                    .concat();
                                let hashed_signed = crypto
                                    .hash_sha256(&raw_signed)
                                    .await
                                    .map_err(|_| ComHubError::SignatureError)?;

                                let ver = crypto
                                    .ver_ed25519(pub_key, &signature, &hashed_signed)
                                    .await
                                    .map_err(|_| ComHubError::SignatureError)?;
                                return Ok(ver);
                            },
                            SignatureType::Unencrypted => {
                                let crypto = get_global_context().crypto;
                                let raw_sign = block.signature
                                    .as_ref()
                                    .ok_or(ComHubError::SignatureError)?;
                                let (signature, pub_key) = raw_sign.split_at(64);

                                let raw_signed = [
                                    pub_key,
                                    &block.body.clone()
                                    ]
                                    .concat();
                                let hashed_signed = crypto
                                    .hash_sha256(&raw_signed)
                                    .await
                                    .map_err(|_| ComHubError::SignatureError)?;

                                let ver = crypto
                                    .ver_ed25519(pub_key, signature, &hashed_signed)
                                    .await
                                    .map_err(|_| ComHubError::SignatureError)?;
                                return Ok(ver);
                            },
                            SignatureType::None => {
                                unreachable!("If (is_signed == true) => !None");
                            }
                        }
                    }
                    else {
                        Ok(true)
                    }
                }
            }
            false => {
                let endpoint = block.routing_header.sender.clone();
                let is_trusted = {
                    cfg_if::cfg_if! {
                        if #[cfg(feature = "debug")] {
                            use crate::runtime::global_context::get_global_context;
                            get_global_context().debug_flags.allow_unsigned_blocks
                        }
                        else {
                            // TODO #181 Check if the sender is trusted (endpoint + interface) connection
                            false
                        }
                    }
                };
                match is_trusted {
                    true => Ok(true),
                    false => {
                        warn!(
                            "Block received by {endpoint} is not signed. Dropping block..."
                        );
                        Ok(false)
                    }
                }
            }
        }
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
    /// This function will wait for all background tasks scheduled by the update() function to finish
    pub async fn update_async(&self) {
        self.update().await;
        self.wait_for_update_async().await;
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
                core::panic!("Interface for uuid {interface_uuid} not found")
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
        let socket =
            self.socket_manager.borrow().get_socket_by_uuid(socket_uuid);
        let socket = socket.try_lock().unwrap();
        self.get_com_interface_by_uuid(&socket.interface_uuid)
    }

    /// Runs the update loop for the ComHub.
    /// This method will continuously handle incoming data, send out
    /// queued blocks and update the sockets.
    /// This is only used for internal tests - in a full runtime setup, the main runtime update loop triggers
    /// ComHub updates.
    pub fn _start_update_loop(self_rc: Rc<Self>) {
        // if already running, do nothing
        if *self_rc.update_loop_running.borrow() {
            return;
        }

        // set update loop running flag
        *self_rc.update_loop_running.borrow_mut() = true;

        spawn_with_panic_notify(
            &self_rc.clone().async_context,
            update_loop_task(self_rc),
        );
    }

    /// Update all sockets and interfaces,
    /// collecting incoming data and sending out queued blocks.
    /// Updates are scheduled in local tasks and are not immediately visible.
    /// To wait for the block update to finish, use `wait_for_update_async()`.
    pub async fn update(&self) {
        // update all interfaces
        self.update_interfaces();
    }

    /// Prepares a block for sending out by updating the creation timestamp,
    /// sender and add signature and encryption if needed.
    /// TODO #379 @Norbert
    pub async fn prepare_own_block(
        &self,
        mut block: DXBBlock,
    ) -> Result<DXBBlock, ComHubError> {
        // TODO #188 signature & encryption
        cfg_if::cfg_if! {
            if #[cfg(feature = "native_crypto")] {
                use crate::runtime::global_context::get_global_context;
                match block.routing_header.flags.signature_type() {
                    SignatureType::Encrypted => {
                        let crypto = get_global_context().crypto;
                        let (pub_key, pri_key) = crypto.gen_ed25519()
                                .await
                                .map_err(|_| ComHubError::SignatureError)?;

                        let raw_signed = [
                            pub_key.clone(),
                            block.body.clone()
                            ]
                            .concat();
                        let hashed_signed = crypto
                            .hash_sha256(&raw_signed)
                            .await
                            .map_err(|_| ComHubError::SignatureError)?;

                        let signature = crypto
                            .sig_ed25519(&pri_key, &hashed_signed)
                            .await
                            .map_err(|_| ComHubError::SignatureError)?;
                        let hash = crypto
                            .hkdf_sha256(&pub_key, &[0u8; 16])
                            .await
                            .map_err(|_| ComHubError::SignatureError)?;
                        let enc_sig = crypto
                            .aes_ctr_encrypt(&hash, &[0u8; 16], &signature)
                            .await
                            .map_err(|_| ComHubError::SignatureError)?;
                        // 64 + 44 = 108
                        block.signature =
                            Some([enc_sig.to_vec(), pub_key].concat());
                    }
                    SignatureType::Unencrypted => {
                        let crypto = get_global_context().crypto;
                        let (pub_key, pri_key) = crypto.gen_ed25519()
                            .await
                            .map_err(|_| ComHubError::SignatureError)?;

                        let raw_signed = [
                            pub_key.clone(),
                            block.body.clone()
                            ]
                            .concat();
                        let hashed_signed = crypto
                            .hash_sha256(&raw_signed)
                            .await
                            .map_err(|_| ComHubError::SignatureError)?;

                        let signature = crypto
                            .sig_ed25519(&pri_key, &hashed_signed)
                            .await
                            .map_err(|_| ComHubError::SignatureError)?;
                        // 64 + 44 = 108
                        block.signature =
                            Some([signature.to_vec(), pub_key].concat());
                    }
                    SignatureType::None => {
                        /* Ignored */
                    }
                }
            }
        }

        let now = Time::now();
        block.routing_header.sender = self.endpoint.clone();
        block
            .block_header
            .flags_and_timestamp
            .set_creation_timestamp(now);

        // set distance to 1
        block.routing_header.distance = 1;
        Ok(block)
    }

    /// Public method to send an outgoing block from this endpoint. Called by the runtime.
    pub async fn send_own_block(
        &self,
        mut block: DXBBlock,
    ) -> Result<(), Vec<Endpoint>> {
        block = self.prepare_own_block(block).await.map_err(|_| vec![])?;
        // add own outgoing block to history
        self.block_handler.add_block_to_history(&block, None);
        self.send_block(block, vec![], false)
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

        let has_exact_receiver_count = block.has_exact_receiver_count();
        let receivers = block.receiver_endpoints();

        let res = self.send_own_block(block).await;
        let failed_endpoints = res.err().unwrap_or_default();

        // yield
        #[cfg(feature = "tokio_runtime")]
        yield_now().await;

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

            let mut rx = self
                .block_handler
                .register_incoming_block_observer(context_id, section_index);

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
                    else if let Some(response) = responses.get_mut(&sender.any_instance_endpoint()) {
                        info!("Received resolved response from {} -> {}", &sender, &sender.any_instance_endpoint());
                        sender = sender.any_instance_endpoint();
                        // check if the receiver is already set (= current set response is Err)
                        if response.is_err() {
                            *response = Ok(Response::ResolvedResponse(sender.clone(), section));
                            missing_response_count -= 1;
                            received_response = true;
                        }
                        // already received a response from a matching endpoint - ignore
                        else {
                            info!("Received multiple resolved responses from the {}", &sender);
                        }
                    }
                    // response from unexpected sender
                    else {
                        error!("Received response from unexpected sender: {}", &sender);
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

    /// Sends a block to all endpoints specified in the block header.
    /// The routing algorithm decides which sockets are used to send the block, based on the endpoint.
    /// A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
    /// The original_socket parameter is used to prevent sending the block back to the sender.
    /// When this method is called, the block is queued in the send queue.
    /// Returns an Err with a list of unreachable endpoints if the block could not be sent to all endpoints.
    pub fn send_block(
        &self,
        mut block: DXBBlock,
        exclude_sockets: Vec<ComInterfaceSocketUUID>,
        forked: bool,
    ) -> Result<(), Vec<Endpoint>> {
        let outbound_receiver_groups =
            self.socket_manager.borrow().get_outbound_receiver_groups(
                &self.endpoint,
                &block.receiver_endpoints(),
                exclude_sockets,
            );

        if outbound_receiver_groups.is_none() {
            error!("No outbound receiver groups found for block");
            return Err(vec![]);
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

        for (receiver_socket, endpoints) in outbound_receiver_groups {
            if let Some(socket_uuid) = receiver_socket {
                self.send_block_to_endpoints_via_socket(
                    block.clone(),
                    &socket_uuid,
                    &endpoints,
                    fork_count,
                );
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

        if !unreachable_endpoints.is_empty() {
            return Err(unreachable_endpoints);
        }
        Ok(())
    }

    /// Sends a block via a socket to a list of endpoints.
    /// Before the block is sent, it is modified to include the list of endpoints as receivers.
    fn send_block_to_endpoints_via_socket(
        &self,
        mut block: DXBBlock,
        socket_uuid: &ComInterfaceSocketUUID,
        endpoints: &[Endpoint],
        // currently only used for trace debugging (TODO: put behind debug flag)
        fork_count: Option<usize>,
    ) {
        block.set_receivers(endpoints);

        // assuming the distance was already increment during redirect, we
        // effectively decrement the block distance by 1 if it is a bounce back
        if block.is_bounce_back() {
            block.routing_header.distance -= 2;
        }

        // if type is Trace or TraceBack, add the outgoing socket to the hops
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
                            self.get_com_interface_from_socket_uuid(
                                socket_uuid,
                            )
                            .borrow_mut()
                            .get_properties(),
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

        let socket =
            self.socket_manager.borrow().get_socket_by_uuid(socket_uuid);
        let mut socket_ref = socket.try_lock().unwrap();

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
        for interceptor in self.outgoing_block_interceptors.borrow().iter() {
            interceptor(&block, socket_uuid, endpoints);
        }
        match &block.to_bytes() {
            Ok(bytes) => {
                info!(
                    "Sending block to socket {}: {}",
                    socket_uuid,
                    endpoints.iter().map(|e| e.to_string()).join(", ")
                );

                // TODO #190: resend block if socket failed to send
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
                        spawn_with_panic_notify(
                            &self.async_context,
                            reconnect_interface_task(interface_rc),
                        );
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

    // /// Collects all blocks from the receive queues of all sockets and process them
    // /// in the receive_block method.
    // async fn receive_incoming_blocks(&self) {
    //     let mut blocks = vec![];
    //     // iterate over all sockets
    //     for (socket, _) in self.sockets.borrow().values() {
    //         let mut socket_ref = socket.try_lock().unwrap();
    //         let uuid = socket_ref.uuid.clone();
    //         let block_queue = socket_ref.get_incoming_block_queue();
    //         blocks.push((uuid, block_queue.drain(..).collect::<Vec<_>>()));
    //     }

    //     for (uuid, blocks) in blocks {
    //         for block in blocks.iter() {
    //             self.receive_block(block, uuid.clone()).await;
    //         }
    //     }
    // }

    // /// Sends all queued blocks from all interfaces.
    // fn flush_outgoing_blocks(&self) {
    //     let interfaces = self.interfaces.borrow();
    //     for (interface, _) in interfaces.values() {
    //         com_interface::flush_outgoing_blocks(
    //             interface.clone(),
    //             &self.async_context,
    //         );
    //     }
    // }
}

#[derive(Default, PartialEq, Debug)]
pub enum ResponseResolutionStrategy {
    /// Promise.allSettled
    /// - For know fixed receivers:
    ///   return after all known sends are finished (either success or error
    ///   if block could not be sent / timed out)
    /// - For unknown receiver count:
    ///   return after timeout
    #[default]
    ReturnAfterAllSettled,

    /// Promise.all
    /// - For know fixed receivers:
    ///   return after all known sends are finished successfully
    ///   return immediately if one send fails early (e.g. endpoint not reachable)
    /// - For unknown receiver count:
    ///   return after timeout
    ///
    ReturnOnAnyError,

    /// Promise.any
    /// Return after first successful response received
    ReturnOnFirstResponse,

    /// Promise.race
    /// Return after first response received (success or error)
    ReturnOnFirstResult,
}

#[derive(Default, Debug)]
pub enum ResponseTimeout {
    #[default]
    Default,
    Custom(Duration),
}

impl ResponseTimeout {
    pub fn unwrap_or_default(self, default: Duration) -> Duration {
        match self {
            ResponseTimeout::Default => default,
            ResponseTimeout::Custom(timeout) => timeout,
        }
    }
}

#[derive(Default, Debug)]
pub struct ResponseOptions {
    pub resolution_strategy: ResponseResolutionStrategy,
    pub timeout: ResponseTimeout,
}

impl ResponseOptions {
    pub fn new_with_resolution_strategy(
        resolution_strategy: ResponseResolutionStrategy,
    ) -> Self {
        Self {
            resolution_strategy,
            ..ResponseOptions::default()
        }
    }

    pub fn new_with_timeout(timeout: Duration) -> Self {
        Self {
            timeout: ResponseTimeout::Custom(timeout),
            ..ResponseOptions::default()
        }
    }
}

#[derive(Debug)]
pub enum Response {
    ExactResponse(Endpoint, IncomingSection),
    ResolvedResponse(Endpoint, IncomingSection),
    UnspecifiedResponse(IncomingSection),
}

impl Response {
    pub fn take_incoming_section(self) -> IncomingSection {
        match self {
            Response::ExactResponse(_, section) => section,
            Response::ResolvedResponse(_, section) => section,
            Response::UnspecifiedResponse(section) => section,
        }
    }
}

#[derive(Debug)]
pub enum ResponseError {
    NoResponseAfterTimeout(Endpoint, Duration),
    NotReachable(Endpoint),
    EarlyAbort(Endpoint),
}

impl Display for ResponseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ResponseError::NoResponseAfterTimeout(endpoint, duration) => {
                core::write!(
                    f,
                    "No response after timeout ({}s) for endpoint {}",
                    duration.as_secs(),
                    endpoint
                )
            }
            ResponseError::NotReachable(endpoint) => {
                core::write!(f, "Endpoint {endpoint} is not reachable")
            }
            ResponseError::EarlyAbort(endpoint) => {
                core::write!(f, "Early abort for endpoint {endpoint}")
            }
        }
    }
}

#[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
async fn handle_com_interface_socket_events(
    mut receiver_queue: UnboundedReceiver<ComInterfaceSocketEvent>,
    socket_manager: Rc<RefCell<SocketManager>>,
    priority: InterfacePriority,
) {
    while let Some(event) = receiver_queue.next().await {
        socket_manager
            .borrow_mut()
            .handle_socket_event(event, priority);
    }
}
