use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use futures::channel::oneshot::Sender;
#[cfg(feature = "native_crypto")]
use crate::crypto::crypto_native::CryptoNative;
use crate::values::core_values::endpoint::Endpoint;
use crate::logger::init_logger;
use crate::stdlib::{cell::RefCell, rc::Rc};
use global_context::{get_global_context, set_global_context, GlobalContext};
use log::info;
use crate::global::dxb_block::{DXBBlock, IncomingSection, IncomingSectionIndex, OutgoingContextId};
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::values::value_container::ValueContainer;
use crate::network::com_hub::{ComHub, ResponseOptions};
use crate::runtime::execution::ExecutionError;
use crate::runtime::execution_context::{ExecutionContext, RemoteExecutionContext, ScriptExecutionError};

pub mod execution;
pub mod global_context;
pub mod memory;
mod stack;
pub mod execution_context;
mod update_loop;

use self::memory::Memory;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Runtime {
    pub version: String,
    pub data: Rc<RuntimeInternal>,
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            version: VERSION.to_string(),
            data: Rc::new(RuntimeInternal::default()),
        }
    }
}

pub struct RuntimeInternal {
    pub memory: RefCell<Memory>,
    pub com_hub: ComHub,
    pub endpoint: Endpoint,
    /// set to true if the update loop should be running
    /// when set to false, the update loop will stop
    update_loop_running: RefCell<bool>,
    update_loop_stop_sender: RefCell<Option<Sender<()>>>,

    /// active execution contexts, stored by context_id
    pub execution_contexts: RefCell<HashMap<u32, ExecutionContext>>,
}

impl Default for RuntimeInternal {
    fn default() -> Self {
        RuntimeInternal {
            endpoint: Endpoint::default(),
            memory: RefCell::new(Memory::new()),
            com_hub: ComHub::default(),
            update_loop_running: RefCell::new(false),
            update_loop_stop_sender: RefCell::new(None),
            execution_contexts: RefCell::new(HashMap::new()),
        }
    }
}


impl RuntimeInternal {
    pub async fn execute(
        self_rc: Rc<RuntimeInternal>,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let dxb = execution_context.compile(script, inserted_values)?;
        RuntimeInternal::execute_dxb(self_rc, dxb, execution_context, true)
            .await
            .map_err(ScriptExecutionError::from)
    }

    pub fn execute_sync(
        self_rc: Rc<RuntimeInternal>,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let dxb = execution_context.compile(script, inserted_values)?;
        RuntimeInternal::execute_dxb_sync(self_rc, &dxb, execution_context, true)
            .map_err(ScriptExecutionError::from)
    }

    pub fn execute_dxb<'a>(
        self_rc: Rc<RuntimeInternal>,
        dxb: Vec<u8>,
        execution_context: &'a mut ExecutionContext,
        end_execution: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ValueContainer>, ExecutionError>> + 'a>> {
        Box::pin(async move {
            match execution_context {
                ExecutionContext::Remote(context) => {
                    RuntimeInternal::execute_remote(self_rc,context, dxb).await
                },
                ExecutionContext::Local(_) => {
                    execution_context.execute_dxb(&dxb, end_execution).await
                }
            }
        })
    }

    pub fn execute_dxb_sync(
        self_rc: Rc<RuntimeInternal>,
        dxb: &[u8],
        execution_context: &mut ExecutionContext,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match execution_context {
            ExecutionContext::Remote(_) => {
                Err(ExecutionError::RequiresAsyncExecution)
            }
            ExecutionContext::Local(_)  => {
                execution_context.execute_dxb_sync(dxb, end_execution)
            }
        }
    }

    /// Returns the existing execution context for the given context_id,
    /// or creates a new one if it doesn't exist.
    fn get_execution_context(
        &self,
        context_id: IncomingSectionIndex,
    ) -> ExecutionContext {
        let mut execution_contexts = self.execution_contexts.borrow_mut();
        // get execution context by context_id or create a new one if it doesn't exist
        let execution_context = execution_contexts.get(&(context_id as u32)).cloned();
        if let Some(context) = execution_context {
            context
        } else {
            let new_context = ExecutionContext::local();
            // insert the new context into the map
            execution_contexts.insert(context_id as u32, new_context.clone());
            new_context
        }
    }

    async fn execute_remote(
        self_rc: Rc<RuntimeInternal>,
        remote_execution_context: &mut RemoteExecutionContext,
        dxb: Vec<u8>
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let routing_header: RoutingHeader = RoutingHeader {
            version: 2,
            flags: routing_header::Flags::new(),
            block_size_u16: Some(0),
            block_size_u32: None,
            sender: self_rc.endpoint.clone(),
            receivers: routing_header::Receivers {
                flags: routing_header::ReceiverFlags::new()
                    .with_has_endpoints(false)
                    .with_has_pointer_id(false)
                    .with_has_endpoint_keys(false),
                pointer_id: None,
                endpoints: None,
                endpoints_with_keys: None,
            },
            ..RoutingHeader::default()
        };

        // get existing context_id for context, or create a new one
        let context_id = remote_execution_context.context_id.unwrap_or_else(|| {
            // if the context_id is not set, we create a new one
            remote_execution_context.context_id = Some(self_rc.com_hub.block_handler.get_new_context_id());
            remote_execution_context.context_id.unwrap()
        });

        let block_header = BlockHeader {
            context_id,
            ..BlockHeader::default()
        };
        let encrypted_header = EncryptedHeader::default();

        let mut block =
            DXBBlock::new(routing_header, block_header, encrypted_header, dxb);

        block.set_receivers(std::slice::from_ref(&remote_execution_context.endpoint));

        let response = self_rc.com_hub.send_own_block_await_response(block, ResponseOptions::default()).await.remove(0)?;
        let incoming_section = response.take_incoming_section();
        RuntimeInternal::execute_incoming_section(self_rc, incoming_section).await.0
    }

    async fn execute_incoming_section(
        self_rc: Rc<RuntimeInternal>,
        incoming_section: IncomingSection,
    ) -> (Result<Option<ValueContainer>, ExecutionError>, Endpoint, OutgoingContextId) {

        let mut context = self_rc.get_execution_context(incoming_section.get_section_index());
        info!("Executing incoming section with index: {}", incoming_section.get_section_index());

        let mut result = None;
        let mut last_block = None;
        for block in incoming_section.into_iter() {
            let res = RuntimeInternal::execute_dxb_block_local(self_rc.clone(), block.clone(), &mut context).await;
            if let Err(err) = res {
                return (Err(err), block.get_sender().clone(), block.block_header.context_id);
            }
            result = res.unwrap();
            last_block = Some(block);
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
        context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        // assert that the execution context is local
        if !matches!(context, ExecutionContext::Local(_)) {
            unreachable!("Execution context must be local for executing a DXB block");
        }
        let dxb = block.body;
        let end_execution = block.block_header.flags_and_timestamp.is_end_of_section();
        RuntimeInternal::execute_dxb(self_rc, dxb, context, end_execution).await
    }
}


/// publicly exposed wrapper impl for the Runtime
/// around RuntimeInternal
impl Runtime {
    pub fn new(endpoint: impl Into<Endpoint>) -> Runtime {
        let endpoint = endpoint.into();
        let com_hub = ComHub::new(endpoint.clone());
        Runtime {
            version: VERSION.to_string(),
            data: Rc::new(RuntimeInternal {
                endpoint,
                com_hub,
                ..RuntimeInternal::default()
            })
        }
    }
    pub fn init(
        endpoint: impl Into<Endpoint>,
        global_context: GlobalContext,
    ) -> Runtime {
        set_global_context(global_context);
        init_logger();
        info!(
            "Runtime initialized - Version {VERSION} Time: {}",
            get_global_context().time.lock().unwrap().now()
        );
        Self::new(endpoint)
    }

    pub fn com_hub(&self) -> &ComHub {
       &self.data.com_hub
    }
    pub fn endpoint(&self) -> Endpoint {
        self.data.endpoint.clone()
    }

    pub fn internal(&self) -> Rc<RuntimeInternal> {
        Rc::clone(&self.data)
    }

    #[cfg(feature = "native_crypto")]
    pub fn init_native(endpoint: impl Into<Endpoint>) -> Runtime {
        use crate::utils::time_native::TimeNative;

        Self::init(
            endpoint,
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
        info!("starting runtime...");
        self.com_hub()
            .init()
            .await
            .expect("Failed to initialize ComHub");
        // ComHub::start_update_loop(self.com_hub());
        RuntimeInternal::start_update_loop(self.internal());
    }
    
    pub async fn execute(
        &self,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        RuntimeInternal::execute(self.internal(), script, inserted_values, execution_context).await
    }

    pub fn execute_sync(
        &self,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        RuntimeInternal::execute_sync(self.internal(), script, inserted_values, execution_context)
    }

    pub async fn execute_dxb<'a>(
        &'a self,
        dxb: Vec<u8>,
        execution_context: &'a mut ExecutionContext,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_dxb(self.internal(), dxb, execution_context, end_execution).await
    }

    pub fn execute_dxb_sync(
        &self,
        dxb: &[u8],
        execution_context: &mut ExecutionContext,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_dxb_sync(self.internal(), dxb, execution_context, end_execution)
    }

    async fn execute_remote(
        &self,
        remote_execution_context: &mut RemoteExecutionContext,
        dxb: Vec<u8>
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_remote(self.internal(), remote_execution_context, dxb).await
    }
}