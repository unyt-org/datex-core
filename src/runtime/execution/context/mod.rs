use log::info;
pub use remote::*;
pub use script::*;
pub use local::*;
use crate::compiler::{compile_template, CompileOptions};
use crate::compiler::error::SpannedCompilerError;
use crate::compiler::scope::CompilationScope;
use crate::runtime::execution::{execute_dxb, execute_dxb_sync, ExecutionError, ExecutionInput, MemoryDump};
use crate::values::value_container::ValueContainer;

mod remote;
mod script;
mod local;

/// An execution context holds the persistent state for executing multiple scripts sequentially within the same context.
/// This can be either a local context, which is used for executing scripts in the same process, or a remote context,
/// which is used for executing scripts on a remote endpoint.
#[derive(Debug, Clone)]
pub enum ExecutionContext {
    Local(LocalExecutionContext),
    Remote(RemoteExecutionContext),
}

impl ExecutionContext {
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
        info!(
            "\x1b[32m[Compiled Bytecode] {}",
            dxb.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(", ")
        );

        #[cfg(feature = "compiler")]
        {
            let decompiled = crate::decompiler::decompile_body(
                dxb,
                crate::decompiler::DecompileOptions::colorized(),
            );
            if let Err(e) = decompiled {
                info!("\x1b[31m[Decompiler Error] {e}\x1b[0m");
            } else {
                let decompiled = decompiled?;
                info!("[Decompiled]: {decompiled}");
            }
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
                                        runtime_execution_state: local_execution_context,
                                        execution_options,
                                        verbose,
                                        ..
                                    }) => (local_execution_context, execution_options, *verbose),
            // remote execution is not supported directly in execution context
            ExecutionContext::Remote(_) => {
                core::panic!("Remote execution requires a Runtime");
            }
        };

        // show DXB and decompiled code if verbose is enabled
        if verbose {
            self.print_dxb_debug(dxb)?;
        }

        local_execution_context.borrow_mut().reset_index();
        Ok(ExecutionInput {
            // FIXME #108: no clone here
            state: (*local_execution_context).clone(),
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
                core::panic!("Remote execution requires a Runtime");
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
