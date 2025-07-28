use std::cell::RefCell;
use std::rc::Rc;
use crate::compiler::error::CompilerError;
use crate::compiler::scope::Scope;
use crate::compiler::{compile_template, CompileOptions};
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use crate::decompiler::{decompile_body, DecompileOptions};
use crate::runtime::execution::{execute_dxb, execute_dxb_sync, ExecutionError, ExecutionInput, ExecutionOptions, RuntimeExecutionContext};

#[derive(Debug)]
pub enum ScriptExecutionError {
    CompilerError(CompilerError),
    ExecutionError(ExecutionError),
}

impl From<CompilerError> for ScriptExecutionError {
    fn from(err: CompilerError) -> Self {
        ScriptExecutionError::CompilerError(err)
    }
}

impl From<ExecutionError> for ScriptExecutionError {
    fn from(err: ExecutionError) -> Self {
        ScriptExecutionError::ExecutionError(err)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RemoteExecutionContext {
    pub compile_scope: Scope,
    pub endpoint: Endpoint,
}

impl RemoteExecutionContext {
    /// Creates a new remote execution context with the given endpoint.
    pub fn new(endpoint: impl Into<Endpoint>) -> Self {
        RemoteExecutionContext {
            compile_scope: Scope::default(),
            endpoint: endpoint.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LocalExecutionContext {
    compile_scope: Scope,
    local_execution_context: Rc<RefCell<RuntimeExecutionContext>>,
    execution_options: ExecutionOptions,
    verbose: bool,
}

impl LocalExecutionContext {
    /// Creates a new local execution context with the given compile scope.
    pub fn debug() -> Self {
        LocalExecutionContext{
            compile_scope: Scope::default(),
            local_execution_context: Rc::new(RefCell::new(RuntimeExecutionContext::default())),
            execution_options: ExecutionOptions {
                verbose: true,
                ..ExecutionOptions::default()
            },
            verbose: true,
        }
    }
}

/// An execution context holds the persistent state for executing multiple scripts sequentially within the same context.
/// This can be either a local context, which is used for executing scripts in the same process, or a remote context,
/// which is used for executing scripts on a remote endpoint.
#[derive(Debug, Clone)]
pub enum ExecutionContext {
    Local(LocalExecutionContext),
    Remote(RemoteExecutionContext)
}

impl ExecutionContext {
    /// Creates a new local execution context.
    pub fn local() -> Self {
        ExecutionContext::Local(LocalExecutionContext::default())
    }

    /// Creates a new local execution context with verbose mode enabled,
    /// providing more log outputs for debugging purposes.
    pub fn local_debug() -> Self {
        ExecutionContext::Local(LocalExecutionContext::debug())
    }

    pub fn remote(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote(RemoteExecutionContext::new(endpoint))
    }

    fn compile_scope(&self) -> &Scope {
        match self {
            ExecutionContext::Local(LocalExecutionContext{ compile_scope, .. }) => compile_scope,
            ExecutionContext::Remote(RemoteExecutionContext{ compile_scope, .. }) => compile_scope,
        }
    }

    fn set_compile_scope(&mut self, new_compile_scope: Scope) {
        match self {
            ExecutionContext::Local(LocalExecutionContext{ compile_scope, .. }) => {
                *compile_scope = new_compile_scope
            }
            ExecutionContext::Remote(RemoteExecutionContext{ compile_scope, .. }) => {
                *compile_scope = new_compile_scope
            }
        }
    }

    /// Compiles a script using the compile scope of the execution context
    pub fn compile(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
    ) -> Result<Vec<u8>, CompilerError> {
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

        let decompiled =
            decompile_body(dxb, DecompileOptions::colorized());
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
        let (local_execution_context, execution_options, verbose) = match &self {
            ExecutionContext::Local(LocalExecutionContext{
                local_execution_context,
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
        if verbose { self.print_dxb_debug(dxb)?; }

        local_execution_context.borrow_mut().reset_index();
        Ok(ExecutionInput {
            // FIXME #108: no clone here
            context: (*local_execution_context).clone(),
            options: (*execution_options).clone(),
            dxb_body: dxb,
            end_execution
        })
    }

    /// Executes DXB in a local execution context.
    pub fn execute_dxb_sync(
        &mut self,
        dxb: &[u8],
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let execution_input = self.get_local_execution_input(dxb, end_execution)?;
        execute_dxb_sync(execution_input)
    }

    /// Executes a script in a local execution context.
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
                let execution_input = self.get_local_execution_input(dxb, end_execution)?;
                execute_dxb(execution_input)
                    .await
            }
            // remote execution is not supported directly in execution context
            ExecutionContext::Remote { .. } => {
                panic!("Remote execution requires a Runtime");
            }
        }
    }

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
}
