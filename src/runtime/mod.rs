#[cfg(feature = "native_crypto")]
use crate::crypto::crypto_native::CryptoNative;
use crate::global::dxb_block::{
    DXBBlock, IncomingEndpointContextSectionId, IncomingSection,
    OutgoingContextId,
};
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::logger::{init_logger, init_logger_debug};
use crate::network::com_hub::{ComHub, InterfacePriority, ResponseOptions};
use crate::runtime::execution::ExecutionError;
use crate::runtime::execution_context::{
    ExecutionContext, RemoteExecutionContext, ScriptExecutionError,
};
use crate::stdlib::{cell::RefCell, rc::Rc};
use crate::values::core_values::endpoint::Endpoint;
use crate::values::serde::serializer::to_value_container;
use crate::values::value_container::ValueContainer;
use datex_core::network::com_interfaces::com_interface::ComInterfaceFactory;
use datex_core::values::serde::error::SerializationError;
use futures::channel::oneshot::Sender;
use global_context::{GlobalContext, get_global_context, set_global_context};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

pub mod execution;
pub mod execution_context;
pub mod global_context;
pub mod memory;
mod stack;
mod update_loop;

use self::memory::Memory;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct Runtime {
    pub version: String,
    pub internal: Rc<RuntimeInternal>,
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("version", &self.version)
            .finish()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            version: VERSION.to_string(),
            internal: Rc::new(RuntimeInternal::default()),
        }
    }
}

#[derive(Debug)]
pub struct RuntimeInternal {
    pub memory: RefCell<Memory>,
    pub com_hub: ComHub,
    pub endpoint: Endpoint,
    pub config: RuntimeConfig,
    /// set to true if the update loop should be running
    /// when set to false, the update loop will stop
    update_loop_running: RefCell<bool>,
    update_loop_stop_sender: RefCell<Option<Sender<()>>>,

    /// active execution contexts, stored by context_id
    pub execution_contexts:
        RefCell<HashMap<IncomingEndpointContextSectionId, ExecutionContext>>,
}

impl Default for RuntimeInternal {
    fn default() -> Self {
        RuntimeInternal {
            endpoint: Endpoint::default(),
            config: RuntimeConfig::default(),
            memory: RefCell::new(Memory::new(Endpoint::default())),
            com_hub: ComHub::default(),
            update_loop_running: RefCell::new(false),
            update_loop_stop_sender: RefCell::new(None),
            execution_contexts: RefCell::new(HashMap::new()),
        }
    }
}

macro_rules! get_execution_context {
    // take context and self_rc as parameters
    ($self_rc:expr, $execution_context:expr) => {
        match $execution_context {
            Some(context) => {
                // set current runtime in execution context if local execution context
                if let &mut ExecutionContext::Local(ref mut local_context) = context {
                    local_context.set_runtime_internal($self_rc.clone());
                }
                context
            },
            None => {
               &mut ExecutionContext::local_with_runtime_internal($self_rc.clone(), true)
            }
        }
    };
}

impl RuntimeInternal {
    pub async fn execute(
        self_rc: Rc<RuntimeInternal>,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: Option<&mut ExecutionContext>,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let execution_context =
            get_execution_context!(self_rc, execution_context);
        let dxb = execution_context.compile(script, inserted_values)?;
        RuntimeInternal::execute_dxb(
            self_rc,
            dxb,
            Some(execution_context),
            true,
        )
        .await
        .map_err(ScriptExecutionError::from)
    }

    pub fn execute_sync(
        self_rc: Rc<RuntimeInternal>,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: Option<&mut ExecutionContext>,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let execution_context =
            get_execution_context!(self_rc, execution_context);
        let dxb = execution_context.compile(script, inserted_values)?;
        RuntimeInternal::execute_dxb_sync(
            self_rc,
            &dxb,
            Some(execution_context),
            true,
        )
        .map_err(ScriptExecutionError::from)
    }

    pub fn execute_dxb<'a>(
        self_rc: Rc<RuntimeInternal>,
        dxb: Vec<u8>,
        execution_context: Option<&'a mut ExecutionContext>,
        end_execution: bool,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<ValueContainer>, ExecutionError>>
                + 'a,
        >,
    > {
        Box::pin(async move {
            let execution_context =
                get_execution_context!(self_rc, execution_context);
            match execution_context {
                ExecutionContext::Remote(context) => {
                    RuntimeInternal::execute_remote(self_rc, context, dxb).await
                }
                ExecutionContext::Local(_) => {
                    execution_context.execute_dxb(&dxb, end_execution).await
                }
            }
        })
    }

    pub fn execute_dxb_sync(
        self_rc: Rc<RuntimeInternal>,
        dxb: &[u8],
        execution_context: Option<&mut ExecutionContext>,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let execution_context =
            get_execution_context!(self_rc, execution_context);
        match execution_context {
            ExecutionContext::Remote(_) => {
                Err(ExecutionError::RequiresAsyncExecution)
            }
            ExecutionContext::Local(_) => {
                execution_context.execute_dxb_sync(dxb, end_execution)
            }
        }
    }

    /// Returns the existing execution context for the given context_id,
    /// or creates a new one if it doesn't exist.
    fn get_execution_context(
        self_rc: Rc<RuntimeInternal>,
        context_id: &IncomingEndpointContextSectionId,
    ) -> ExecutionContext {
        let mut execution_contexts = self_rc.execution_contexts.borrow_mut();
        // get execution context by context_id or create a new one if it doesn't exist
        let execution_context = execution_contexts.get(context_id).cloned();
        if let Some(context) = execution_context {
            context
        } else {
            let new_context = ExecutionContext::local_with_runtime_internal(
                self_rc.clone(),
                false,
            );
            // insert the new context into the map
            execution_contexts.insert(context_id.clone(), new_context.clone());
            new_context
        }
    }

    pub async fn execute_remote(
        self_rc: Rc<RuntimeInternal>,
        remote_execution_context: &mut RemoteExecutionContext,
        dxb: Vec<u8>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let routing_header: RoutingHeader = RoutingHeader {
            version: 2,
            flags: routing_header::Flags::new(),
            block_size_u16: Some(0),
            block_size_u32: None,
            sender: self_rc.endpoint.clone(),
            ..RoutingHeader::default()
        };

        // get existing context_id for context, or create a new one
        let context_id =
            remote_execution_context.context_id.unwrap_or_else(|| {
                // if the context_id is not set, we create a new one
                remote_execution_context.context_id =
                    Some(self_rc.com_hub.block_handler.get_new_context_id());
                remote_execution_context.context_id.unwrap()
            });

        let block_header = BlockHeader {
            context_id,
            ..BlockHeader::default()
        };
        let encrypted_header = EncryptedHeader::default();

        let mut block =
            DXBBlock::new(routing_header, block_header, encrypted_header, dxb);

        block.set_receivers(std::slice::from_ref(
            &remote_execution_context.endpoint,
        ));

        let response = self_rc
            .com_hub
            .send_own_block_await_response(block, ResponseOptions::default())
            .await
            .remove(0)?;
        let incoming_section = response.take_incoming_section();
        RuntimeInternal::execute_incoming_section(self_rc, incoming_section)
            .await
            .0
    }

    async fn execute_incoming_section(
        self_rc: Rc<RuntimeInternal>,
        mut incoming_section: IncomingSection,
    ) -> (
        Result<Option<ValueContainer>, ExecutionError>,
        Endpoint,
        OutgoingContextId,
    ) {
        let mut context = Self::get_execution_context(
            self_rc.clone(),
            incoming_section.get_section_context_id(),
        );
        info!(
            "Executing incoming section with index: {}",
            incoming_section.get_section_index()
        );

        let mut result = None;
        let mut last_block = None;

        // iterate over the blocks in the incoming section
        loop {
            let block = incoming_section.next().await;
            if let Some(block) = block {
                let res = RuntimeInternal::execute_dxb_block_local(
                    self_rc.clone(),
                    block.clone(),
                    Some(&mut context),
                )
                .await;
                if let Err(err) = res {
                    return (
                        Err(err),
                        block.get_sender().clone(),
                        block.block_header.context_id,
                    );
                }
                result = res.unwrap();
                last_block = Some(block);
            } else {
                break;
            }
        }

        if last_block.is_none() {
            unreachable!("Incoming section must contain at least one block");
        }
        let last_block = last_block.unwrap();
        let sender_endpoint = last_block.get_sender().clone();
        let context_id = last_block.block_header.context_id;
        (Ok(result), sender_endpoint, context_id)
    }

    async fn execute_dxb_block_local(
        self_rc: Rc<RuntimeInternal>,
        block: DXBBlock,
        execution_context: Option<&mut ExecutionContext>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let execution_context =
            get_execution_context!(self_rc, execution_context);
        // assert that the execution context is local
        if !matches!(execution_context, ExecutionContext::Local(_)) {
            unreachable!(
                "Execution context must be local for executing a DXB block"
            );
        }
        let dxb = block.body;
        let end_execution =
            block.block_header.flags_and_timestamp.is_end_of_section();
        RuntimeInternal::execute_dxb(
            self_rc,
            dxb,
            Some(execution_context),
            end_execution,
        )
        .await
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RuntimeConfigInterface {
    r#type: String,
    config: ValueContainer,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub endpoint: Option<Endpoint>,
    pub interfaces: Option<Vec<RuntimeConfigInterface>>,
    /// if set to true, the runtime will log debug messages
    pub debug: Option<bool>,
}

impl RuntimeConfig {
    pub fn new_with_endpoint(endpoint: Endpoint) -> Self {
        RuntimeConfig {
            endpoint: Some(endpoint),
            interfaces: None,
            debug: None,
        }
    }

    pub fn add_interface<T: Serialize>(
        &mut self,
        r#type: String,
        config: T,
    ) -> Result<(), SerializationError> {
        let config = to_value_container(&config)?;
        let interface = RuntimeConfigInterface { r#type, config };
        if let Some(interfaces) = &mut self.interfaces {
            interfaces.push(interface);
        } else {
            self.interfaces = Some(vec![interface]);
        }

        Ok(())
    }
}

/// publicly exposed wrapper impl for the Runtime
/// around RuntimeInternal
impl Runtime {
    pub fn new(config: RuntimeConfig) -> Runtime {
        let endpoint = config.endpoint.clone().unwrap_or_else(Endpoint::random);
        let com_hub = ComHub::new(endpoint.clone());
        let memory = RefCell::new(Memory::new(endpoint.clone()));
        Runtime {
            version: VERSION.to_string(),
            internal: Rc::new(RuntimeInternal {
                endpoint,
                memory,
                config,
                com_hub,
                ..RuntimeInternal::default()
            }),
        }
    }

    pub fn init(
        config: RuntimeConfig,
        global_context: GlobalContext,
    ) -> Runtime {
        set_global_context(global_context);
        if let Some(debug) = config.debug
            && debug
        {
            init_logger_debug();
        } else {
            init_logger();
        }
        info!(
            "Runtime initialized - Version {VERSION} Time: {}",
            get_global_context().time.lock().unwrap().now()
        );
        Self::new(config)
    }

    pub fn com_hub(&self) -> &ComHub {
        &self.internal.com_hub
    }
    pub fn endpoint(&self) -> Endpoint {
        self.internal.endpoint.clone()
    }

    pub fn internal(&self) -> Rc<RuntimeInternal> {
        Rc::clone(&self.internal)
    }

    pub fn memory(&self) -> &RefCell<Memory> {
        &self.internal.memory
    }

    #[cfg(feature = "native_crypto")]
    pub fn init_native(config: RuntimeConfig) -> Runtime {
        use crate::utils::time_native::TimeNative;

        Self::init(
            config,
            GlobalContext::new(
                Arc::new(Mutex::new(CryptoNative)),
                Arc::new(Mutex::new(TimeNative)),
            ),
        )
    }

    /// Starts the common update loop:
    ///  - ComHub
    ///  - Runtime
    pub async fn start(&self) {
        if *self.internal().update_loop_running.borrow() {
            info!("runtime update loop already running, skipping start");
            return;
        }
        info!("starting runtime...");
        self.com_hub()
            .init()
            .await
            .expect("Failed to initialize ComHub");

        // register interface factories
        self.register_interface_factories();

        // create interfaces
        if let Some(interfaces) = &self.internal.config.interfaces {
            for RuntimeConfigInterface { r#type, config } in interfaces.iter() {
                if let Err(err) = self
                    .com_hub()
                    .create_interface(
                        r#type,
                        config.clone(),
                        InterfacePriority::default(),
                    )
                    .await
                {
                    error!("Failed to create interface {type}: {err:?}");
                } else {
                    info!("Created interface: {type}");
                }
            }
        }

        RuntimeInternal::start_update_loop(self.internal());
    }

    // inits a runtime and starts the update loop
    pub async fn create(
        config: RuntimeConfig,
        global_context: GlobalContext,
    ) -> Runtime {
        let runtime = Self::init(config, global_context);
        runtime.start().await;
        runtime
    }

    // inits a native runtime and starts the update loop
    #[cfg(feature = "native_crypto")]
    pub async fn create_native(config: RuntimeConfig) -> Runtime {
        let runtime = Self::init_native(config);
        runtime.start().await;
        runtime
    }

    fn register_interface_factories(&self) {
        crate::network::com_interfaces::default_com_interfaces::base_interface::BaseInterface::register_on_com_hub(self.com_hub());

        #[cfg(feature = "native_websocket")]
        crate::network::com_interfaces::default_com_interfaces::websocket::websocket_client_native_interface::WebSocketClientNativeInterface::register_on_com_hub(self.com_hub());
        #[cfg(feature = "native_websocket")]
        crate::network::com_interfaces::default_com_interfaces::websocket::websocket_server_native_interface::WebSocketServerNativeInterface::register_on_com_hub(self.com_hub());
        #[cfg(feature = "native_serial")]
        crate::network::com_interfaces::default_com_interfaces::serial::serial_native_interface::SerialNativeInterface::register_on_com_hub(self.com_hub());
        #[cfg(feature = "native_tcp")]
        crate::network::com_interfaces::default_com_interfaces::tcp::tcp_client_native_interface::TCPClientNativeInterface::register_on_com_hub(self.com_hub());
        #[cfg(feature = "native_tcp")]
        crate::network::com_interfaces::default_com_interfaces::tcp::tcp_server_native_interface::TCPServerNativeInterface::register_on_com_hub(self.com_hub());
        // TODO #234:
        // #[cfg(feature = "native_webrtc")]
        // crate::network::com_interfaces::default_com_interfaces::webrtc::webrtc_native_interface::WebRTCNativeInterface::register_on_com_hub(self.com_hub());
    }

    pub async fn execute(
        &self,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: Option<&mut ExecutionContext>,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        RuntimeInternal::execute(
            self.internal(),
            script,
            inserted_values,
            execution_context,
        )
        .await
    }

    pub fn execute_sync(
        &self,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: Option<&mut ExecutionContext>,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        RuntimeInternal::execute_sync(
            self.internal(),
            script,
            inserted_values,
            execution_context,
        )
    }

    pub async fn execute_dxb<'a>(
        &'a self,
        dxb: Vec<u8>,
        execution_context: Option<&'a mut ExecutionContext>,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_dxb(
            self.internal(),
            dxb,
            execution_context,
            end_execution,
        )
        .await
    }

    pub fn execute_dxb_sync(
        &self,
        dxb: &[u8],
        execution_context: Option<&mut ExecutionContext>,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_dxb_sync(
            self.internal(),
            dxb,
            execution_context,
            end_execution,
        )
    }

    async fn execute_remote(
        &self,
        remote_execution_context: &mut RemoteExecutionContext,
        dxb: Vec<u8>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_remote(
            self.internal(),
            remote_execution_context,
            dxb,
        )
        .await
    }
}
