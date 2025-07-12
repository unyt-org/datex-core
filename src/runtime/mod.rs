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
use crate::global::dxb_block::{DXBBlock, IncomingSection};
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
    pub memory: Rc<RefCell<Memory>>,
    pub com_hub: Rc<ComHub>,
    pub endpoint: Endpoint,
    /// set to true if the update loop should be running
    /// when set to false, the update loop will stop
    update_loop_running: RefCell<bool>,
    update_loop_stop_sender: RefCell<Option<Sender<()>>>,
}

impl Runtime {
    pub fn new(endpoint: impl Into<Endpoint>) -> Runtime {
        let endpoint = endpoint.into();
        let com_hub = ComHub::new(endpoint.clone());
        Runtime {
            endpoint,
            com_hub: Rc::new(com_hub),
            ..Runtime::default()
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
    pub async fn start(self_rc: Rc<Runtime>) {
        info!("starting runtime...");
        self_rc.com_hub
            .init()
            .await
            .expect("Failed to initialize ComHub");
        ComHub::start_update_loop(self_rc.com_hub.clone());
        Runtime::start_update_loop(self_rc.clone());
    }    
    
    pub async fn execute(
        &self,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let dxb = execution_context.compile(script, inserted_values)?;
        self.execute_dxb(dxb, execution_context, true)
            .await
            .map_err(ScriptExecutionError::from)
    }

    pub fn execute_dxb<'a>(
        &'a self,
        dxb: Vec<u8>,
        execution_context: &'a mut ExecutionContext,
        end_execution: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ValueContainer>, ExecutionError>> + 'a>> {
        Box::pin(async move {
            match execution_context {
                ExecutionContext::Remote(context) => {
                    self.execute_remote(context, dxb).await
                },
                ExecutionContext::Local(_) => {
                    execution_context.execute_dxb(&dxb, end_execution).await
                }
            }
        })
    }

    pub fn execute_sync(
        &self,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let dxb = execution_context.compile(script, inserted_values)?;
        self.execute_dxb_sync(&dxb, execution_context, true)
            .map_err(ScriptExecutionError::from)
    }

    pub fn execute_dxb_sync(
        &self,
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

    async fn execute_remote(&self, remote_execution_context: &mut RemoteExecutionContext, dxb: Vec<u8>) -> Result<Option<ValueContainer>, ExecutionError> {
        let routing_header: RoutingHeader = RoutingHeader {
            version: 2,
            flags: routing_header::Flags::new(),
            block_size_u16: Some(0),
            block_size_u32: None,
            sender: self.endpoint.clone(),
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

        let response = self.com_hub.send_own_block_await_response(block, ResponseOptions::default()).await.remove(0)?;
        let incoming_section = response.take_incoming_section();

        let mut context = ExecutionContext::local();

        self.execute_incoming_section(incoming_section, &mut context).await
    }

    async fn execute_incoming_section(
        &self,
        incoming_section: IncomingSection,
        context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let mut result = None;
        for block in incoming_section.into_iter() {
            result = self.execute_dxb_block_local(block, context).await?;
        }
        Ok(result)
    }

    async fn execute_dxb_block_local(
        &self,
        block: DXBBlock,
        context: &mut ExecutionContext,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        // assert that the execution context is local
        if !matches!(context, ExecutionContext::Local(_)) {
            unreachable!("Execution context must be local for executing a DXB block");
        }
        let dxb = block.body;
        let end_execution = block.block_header.flags_and_timestamp.is_end_of_section();
        self.execute_dxb(dxb, context, end_execution).await
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            endpoint: Endpoint::default(),
            version: VERSION.to_string(),
            memory: Rc::new(RefCell::new(Memory::new())),
            com_hub: Rc::new(ComHub::default()),
            update_loop_running: RefCell::new(false),
            update_loop_stop_sender: RefCell::new(None),
        }
    }
}
