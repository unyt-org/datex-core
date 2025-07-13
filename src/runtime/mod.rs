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
use crate::global::dxb_block::{DXBBlock, IncomingSection, IncomingSectionIndex};
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

pub struct RuntimeInternal {
    pub memory: RefCell<Memory>,
    pub com_hub: ComHub,
    pub endpoint: Endpoint,
    /// set to true if the update loop should be running
    /// when set to false, the update loop will stop
    update_loop_running: RefCell<bool>,
    update_loop_stop_sender: RefCell<Option<Sender<()>>>,
}

impl Default for RuntimeInternal {
    fn default() -> Self {
        RuntimeInternal {
            endpoint: Endpoint::default(),
            memory: RefCell::new(Memory::new()),
            com_hub: ComHub::default(),
            update_loop_running: RefCell::new(false),
            update_loop_stop_sender: RefCell::new(None),
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

        let block_header = BlockHeader::default();
        let encrypted_header = EncryptedHeader::default();

        let mut block =
            DXBBlock::new(routing_header, block_header, encrypted_header, dxb);

        block.set_receivers(std::slice::from_ref(&remote_execution_context.endpoint));

        let response = self_rc.com_hub.send_own_block_await_response(block, ResponseOptions::default()).await.remove(0)?;
        let incoming_section = response.take_incoming_section();

        let mut context = ExecutionContext::local();

        RuntimeInternal::execute_incoming_section(self_rc, incoming_section, &mut context).await.0
    }

    async fn execute_incoming_section(
        self_rc: Rc<RuntimeInternal>,
        incoming_section: IncomingSection,
        context: &mut ExecutionContext,
    ) -> (Result<Option<ValueContainer>, ExecutionError>, Endpoint, IncomingSectionIndex) {
        let mut result = None;
        let mut last_block = None;
        for block in incoming_section.into_iter() {
            let res = RuntimeInternal::execute_dxb_block_local(self_rc.clone(), block.clone(), context).await;
            if let Err(err) = res {
                return (Err(err), block.get_sender().clone(), block.block_header.section_index);
            }
            result = res.unwrap();
            last_block = Some(block);
        }
        if last_block.is_none() {
            unreachable!("Incoming section must contain at least one block");
        }
        let last_block = last_block.unwrap();
        let sender_endpoint = last_block.get_sender().clone();
        let section_index = last_block.block_header.section_index;
        (Ok(result), sender_endpoint, section_index)
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