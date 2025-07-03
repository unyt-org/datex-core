use crate::compiler::error::CompilerError;
use crate::compiler::scope::Scope;
use crate::compiler::{compile_template, CompileOptions};
use crate::datex_values::core_values::endpoint::Endpoint;
use crate::datex_values::value_container::ValueContainer;
use crate::decompiler::{decompile_body, DecompileOptions};
use crate::runtime::execution::{
    execute_dxb, ExecutionError, ExecutionInput, ExecutionOptions,
    LocalExecutionContext,
};

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

/// An execution context holds the persistent state for executing multiple scripts sequentially within the same context.
/// This can be either a local context, which is used for executing scripts in the same process, or a remote context,
/// which is used for executing scripts on a remote endpoint.
#[derive(Debug, Clone)]
pub enum ExecutionContext {
    Local {
        compile_scope: Scope,
        local_execution_context: LocalExecutionContext,
        execution_options: ExecutionOptions,
        verbose: bool,
    },

    Remote {
        compile_scope: Scope,
        endpoint: Endpoint,
    },
}

impl ExecutionContext {
    /// Creates a new local execution context.
    pub fn local() -> Self {
        ExecutionContext::Local {
            compile_scope: Scope::default(),
            local_execution_context: LocalExecutionContext::default(),
            execution_options: ExecutionOptions::default(),
            verbose: false,
        }
    }

    /// Creates a new local execution context with verbose mode enabled,
    /// providing more log outputs for debugging purposes.
    pub fn local_debug() -> Self {
        ExecutionContext::Local {
            compile_scope: Scope::default(),
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
            compile_scope: Scope::default(),
            endpoint: endpoint.into(),
        }
    }

    fn compile_scope(&self) -> &Scope {
        match self {
            ExecutionContext::Local { compile_scope, .. } => compile_scope,
            ExecutionContext::Remote { compile_scope, .. } => compile_scope,
        }
    }

    fn set_compile_scope(&mut self, new_compile_scope: Scope) {
        match self {
            ExecutionContext::Local { compile_scope, .. } => {
                *compile_scope = new_compile_scope
            }
            ExecutionContext::Remote { compile_scope, .. } => {
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
        // TODO: don't clone compile_scope if possible
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

    /// Executes DXB in a local execution context.
    pub fn execute_dxb_local(
        &mut self,
        dxb: &[u8],
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let (local_execution_context, execution_options, verbose) = match self {
            ExecutionContext::Local {
                local_execution_context,
                execution_options,
                verbose,
                ..
            } => (local_execution_context, execution_options, *verbose),
            ExecutionContext::Remote { .. } => {
                panic!("Cannot run execute_dxb_local on a remote execution context. Use execute_dxb_remote instead.");
            }
        };

        // show DXB and decompiled code if verbose is enabled
        if verbose {
            println!(
                "\x1b[32m[Compiled Bytecode] {}",
                dxb.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            let decompiled =
                decompile_body(&dxb, DecompileOptions::colorized());
            if let Err(e) = decompiled {
                println!("\x1b[31m[Decompiler Error] {e}\x1b[0m");
            } else {
                let decompiled = decompiled.unwrap();
                println!("[Decompiled]: {}", decompiled);
            }
        }

        local_execution_context.reset_index();
        let execution_input = ExecutionInput {
            // FIXME: no clone here
            context: local_execution_context.clone(),
            options: execution_options.clone(),
            dxb_body: dxb,
        };
        let res = execute_dxb(execution_input);
        match res {
            Ok((result, new_context)) => {
                *local_execution_context = new_context;
                Ok(result)
            }
            Err(err) => Err(err),
        }
    }

    /// Executes a script in a local execution context.
    pub fn execute_local(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let dxb = self.compile(script, inserted_values)?;
        self.execute_dxb_local(&dxb)
            .map_err(ScriptExecutionError::ExecutionError)
    }
}
