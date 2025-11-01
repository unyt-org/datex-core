#[cfg(feature = "compiler")]
use crate::compiler::{
    error::SpannedCompilerError,
    scope::CompilationScope,
    CompileOptions, compile_template
};
use crate::decompiler::{DecompileOptions, decompile_body};
use crate::global::dxb_block::OutgoingContextId;
use crate::runtime::RuntimeInternal;
use crate::runtime::execution::{
    ExecutionError, ExecutionInput, ExecutionOptions, MemoryDump,
    RuntimeExecutionContext, execute_dxb, execute_dxb_sync,
};
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;
use core::fmt::Display;
use crate::stdlib::rc::Rc;

#[derive(Debug)]
pub enum ScriptExecutionError {
    #[cfg(feature = "compiler")]
    CompilerError(SpannedCompilerError),
    ExecutionError(ExecutionError),
}

#[cfg(feature = "compiler")]
impl From<SpannedCompilerError> for ScriptExecutionError {
    fn from(err: SpannedCompilerError) -> Self {
        ScriptExecutionError::CompilerError(err)
    }
}

impl From<ExecutionError> for ScriptExecutionError {
    fn from(err: ExecutionError) -> Self {
        ScriptExecutionError::ExecutionError(err)
    }
}

impl Display for ScriptExecutionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ScriptExecutionError::CompilerError(err) => {
                core::write!(f, "Compiler Error: {}", err)
            }
            ScriptExecutionError::ExecutionError(err) => {
                core::write!(f, "Execution Error: {}", err)
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RemoteExecutionContext {
    pub compile_scope: CompilationScope,
    pub endpoint: Endpoint,
    pub context_id: Option<OutgoingContextId>,
}

impl RemoteExecutionContext {
    /// Creates a new remote execution context with the given endpoint.
    pub fn new(endpoint: impl Into<Endpoint>, once: bool) -> Self {
        RemoteExecutionContext {
            compile_scope: CompilationScope::new(once),
            endpoint: endpoint.into(),
            context_id: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LocalExecutionContext {
    compile_scope: CompilationScope,
    runtime_execution_context: Rc<RefCell<RuntimeExecutionContext>>,
    execution_options: ExecutionOptions,
    verbose: bool,
}

impl LocalExecutionContext {
    pub fn new(once: bool) -> Self {
        LocalExecutionContext {
            compile_scope: CompilationScope::new(once),
            runtime_execution_context: Rc::new(RefCell::new(
                RuntimeExecutionContext::default(),
            )),
            execution_options: ExecutionOptions::default(),
            verbose: false,
        }
    }

    /// Creates a new local execution context with the given compile scope.
    pub fn debug(once: bool) -> Self {
        LocalExecutionContext {
            compile_scope: CompilationScope::new(once),
            execution_options: ExecutionOptions { verbose: true },
            verbose: true,
            ..Default::default()
        }
    }

    pub fn debug_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        LocalExecutionContext {
            compile_scope: CompilationScope::new(once),
            runtime_execution_context: Rc::new(RefCell::new(
                RuntimeExecutionContext::new(runtime_internal),
            )),
            execution_options: ExecutionOptions { verbose: true },
            verbose: true,
        }
    }

    pub fn new_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        LocalExecutionContext {
            compile_scope: CompilationScope::new(once),
            runtime_execution_context: Rc::new(RefCell::new(
                RuntimeExecutionContext::new(runtime_internal),
            )),
            ..Default::default()
        }
    }

    pub fn set_runtime_internal(
        &mut self,
        runtime_internal: Rc<RuntimeInternal>,
    ) {
        self.runtime_execution_context
            .borrow_mut()
            .set_runtime_internal(runtime_internal);
    }

    /// Returns a memory dump of the current state of the execution context.
    pub fn memory_dump(&self) -> MemoryDump {
        self.runtime_execution_context.borrow().memory_dump()
    }
}

/// An execution context holds the persistent state for executing multiple scripts sequentially within the same context.
/// This can be either a local context, which is used for executing scripts in the same process, or a remote context,
/// which is used for executing scripts on a remote endpoint.
#[derive(Debug, Clone)]
pub enum ExecutionContext {
    Local(LocalExecutionContext),
    Remote(RemoteExecutionContext),
}

impl ExecutionContext {
    /// Creates a new local execution context (can only be used once).
    pub fn local_once() -> Self {
        ExecutionContext::Local(LocalExecutionContext::new(true))
    }

    /// Creates a new local execution context (can be used multiple times).
    pub fn local() -> Self {
        ExecutionContext::Local(LocalExecutionContext::new(false))
    }

    /// Creates a new local execution context with a runtime.
    pub fn local_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        ExecutionContext::Local(
            LocalExecutionContext::new_with_runtime_internal(
                runtime_internal,
                once,
            ),
        )
    }

    /// Creates a new local execution context with verbose mode enabled,
    /// providing more log outputs for debugging purposes.
    pub fn local_debug(once: bool) -> Self {
        ExecutionContext::Local(LocalExecutionContext::debug(once))
    }

    /// Creates a new local execution context with verbose mode enabled and a runtime.
    pub fn local_debug_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        ExecutionContext::Local(
            LocalExecutionContext::debug_with_runtime_internal(
                runtime_internal,
                once,
            ),
        )
    }

    pub fn remote_once(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote(RemoteExecutionContext::new(endpoint, true))
    }

    pub fn remote(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote(RemoteExecutionContext::new(endpoint, false))
    }

    #[cfg(feature = "compiler")]
    fn compile_scope(&self) -> &CompilationScope {
        match self {
            ExecutionContext::Local(LocalExecutionContext {
                compile_scope,
                ..
            }) => compile_scope,
            ExecutionContext::Remote(RemoteExecutionContext {
                compile_scope,
                ..
            }) => compile_scope,
        }
    }

    #[cfg(feature = "compiler")]
    fn set_compile_scope(&mut self, new_compile_scope: CompilationScope) {
        match self {
            ExecutionContext::Local(LocalExecutionContext {
                compile_scope,
                ..
            }) => *compile_scope = new_compile_scope,
            ExecutionContext::Remote(RemoteExecutionContext {
                compile_scope,
                ..
            }) => *compile_scope = new_compile_scope,
        }
    }

    /// Compiles a script using the compile scope of the execution context
    #[cfg(feature = "compiler")]
    pub fn compile(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
    ) -> Result<Vec<u8>, SpannedCompilerError> {
        let compile_scope = self.compile_scope();
        // TODO #107: don't clone compile_scope if possible
        let res = compile_template(
            script,
            inserted_values,
            CompileOptions::new_with_scope(compile_scope.clone()),
        );
        match res {
            Ok((bytes, compile_scope)) => {
                self.set_compile_scope(compile_scope);
                Ok(bytes)
            }
            Err(err) => Err(err),
        }
    }

    fn print_dxb_debug(&self, dxb: &[u8]) -> Result<(), ExecutionError> {
        println!(
            "\x1b[32m[Compiled Bytecode] {}",
            dxb.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let decompiled = decompile_body(dxb, DecompileOptions::colorized());
        if let Err(e) = decompiled {
            println!("\x1b[31m[Decompiler Error] {e}\x1b[0m");
        } else {
            let decompiled = decompiled?;
            println!("[Decompiled]: {decompiled}");
        }

        Ok(())
    }

    fn get_local_execution_input<'a>(
        &'a mut self,
        dxb: &'a [u8],
        end_execution: bool,
    ) -> Result<ExecutionInput<'a>, ExecutionError> {
        let (local_execution_context, execution_options, verbose) = match &self
        {
            ExecutionContext::Local(LocalExecutionContext {
                runtime_execution_context: local_execution_context,
                execution_options,
                verbose,
                ..
            }) => (local_execution_context, execution_options, *verbose),
            // remote execution is not supported directly in execution context
            ExecutionContext::Remote(_) => {
                panic!("Remote execution requires a Runtime");
            }
        };

        // show DXB and decompiled code if verbose is enabled
        if verbose {
            self.print_dxb_debug(dxb)?;
        }

        local_execution_context.borrow_mut().reset_index();
        Ok(ExecutionInput {
            // FIXME #108: no clone here
            context: (*local_execution_context).clone(),
            options: (*execution_options).clone(),
            dxb_body: dxb,
            end_execution,
        })
    }

    /// Executes DXB in a local execution context.
    pub fn execute_dxb_sync(
        &mut self,
        dxb: &[u8],
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let execution_input =
            self.get_local_execution_input(dxb, end_execution)?;
        execute_dxb_sync(execution_input)
    }

    /// Executes a script in a local execution context.
    #[cfg(feature = "compiler")]
    pub fn execute_sync(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let dxb = self.compile(script, inserted_values)?;
        self.execute_dxb_sync(&dxb, true)
            .map_err(ScriptExecutionError::from)
    }

    pub async fn execute_dxb(
        &mut self,
        dxb: &[u8],
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match self {
            ExecutionContext::Local { .. } => {
                let execution_input =
                    self.get_local_execution_input(dxb, end_execution)?;
                execute_dxb(execution_input).await
            }
            // remote execution is not supported directly in execution context
            ExecutionContext::Remote { .. } => {
                panic!("Remote execution requires a Runtime");
            }
        }
    }

    #[cfg(feature = "compiler")]
    pub async fn execute(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let dxb = self.compile(script, inserted_values)?;
        self.execute_dxb(&dxb, true)
            .await
            .map_err(ScriptExecutionError::from)
    }

    /// Returns a memory dump of the current state of the execution context if available.
    pub fn memory_dump(&self) -> Option<MemoryDump> {
        match self {
            ExecutionContext::Local(local_context) => {
                Some(local_context.memory_dump())
            }
            // TODO #397: also support remote memory dump if possible
            ExecutionContext::Remote(_) => None,
        }
    }
}
