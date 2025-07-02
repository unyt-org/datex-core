use crate::compiler::bytecode::{compile_template, CompileOptions, CompileScope};
use crate::compiler::CompilerError;
use crate::datex_values::core_values::endpoint::Endpoint;
use crate::datex_values::value_container::ValueContainer;
use crate::runtime::execution::{execute_dxb, ExecutionError, ExecutionInput, ExecutionOptions, LocalExecutionContext};

/// An execution context holds the persistent state for executing multiple scripts sequentially within the same context.
/// This can be either a local context, which is used for executing scripts in the same process, or a remote context, 
/// which is used for executing scripts on a remote endpoint.
#[derive(Debug, Clone)]
pub enum ExecutionContext {
    Local {
        compile_scope: CompileScope,
        local_execution_context: LocalExecutionContext,
        execution_options: ExecutionOptions,
        verbose: bool,
    },

    Remote {
        compile_scope: CompileScope,
        endpoint: Endpoint,
    },
}

impl ExecutionContext {
    /// Creates a new local execution context.
    pub fn local() -> Self {
        ExecutionContext::Local {
            compile_scope: CompileScope::default(),
            local_execution_context: LocalExecutionContext::default(),
            execution_options: ExecutionOptions::default(),
            verbose: false,
        }
    }
    
    /// Creates a new local execution context with verbose mode enabled,
    /// providing more log outputs for debugging purposes.
    pub fn local_debug() -> Self {
        ExecutionContext::Local {
            compile_scope: CompileScope::default(),
            local_execution_context: LocalExecutionContext::default(),
            execution_options: ExecutionOptions {
                verbose: true,
                ..ExecutionOptions::default()
            },
            verbose: true,
        }
    }

    pub fn remote(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote {
            compile_scope: CompileScope::default(),
            endpoint: endpoint.into(),
        }
    }
    
    fn compile_scope(&self) -> &CompileScope {
        match self {
            ExecutionContext::Local { compile_scope, .. } => compile_scope,
            ExecutionContext::Remote { compile_scope, .. } => compile_scope,
        }
    }
    
    
    /// Compiles a script using the compile scope of the execution context
    pub fn compile(&mut self, script: &str, inserted_values: &[ValueContainer]) -> Result<Vec<u8>, CompilerError> {
        let compile_scope = self.compile_scope();
        // TODO: don't clone compile_scope if possible
        let res = compile_template(script, inserted_values, CompileOptions::new_with_scope(compile_scope.clone()));
        match res {
            Ok((bytes, ..)) => Ok(bytes),
            Err(err) => Err(err),
        }
    }

    /// Executes a script in a local execution context.
    pub fn execute_dxb_local(&mut self, dxb: &[u8]) -> Result<Option<ValueContainer>, ExecutionError> {
        let (local_execution_context, execution_options) = match self {
            ExecutionContext::Local { local_execution_context, execution_options, .. } => (local_execution_context, execution_options),
            ExecutionContext::Remote { .. } => {
                panic!("Cannot run execute_dxb_local on a remote execution context. Use execute_dxb_remote instead.");
            }
        };
        local_execution_context.reset_index();
        let execution_input = ExecutionInput {
            // FIXME: no clone here
            context: local_execution_context.clone(),
            options: execution_options.clone(),
            dxb_body: dxb
        };
        let res = execute_dxb(execution_input);
        match res {
            Ok((result, new_context)) => {
                *local_execution_context = new_context;
                Ok(result)
            },
            Err(err) => Err(err),
        }
    }
}