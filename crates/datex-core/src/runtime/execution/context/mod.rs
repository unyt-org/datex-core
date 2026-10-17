//! This module contains the implementation of the execution context, which holds the state of an active execution.
#[cfg(feature = "compiler")]
use crate::compiler::{
    CompileOptions, compile_template, error::SpannedCompilerError,
    scope::CompilationScope,
};
use crate::{
    core_compiler::core_compilation_context::DXBWithSharedValues,
    runtime::execution::{
        ExecutionError, ExecutionInput, MemoryDump, execute_dxb,
        execute_dxb_sync,
    },
    values::{
        core_values::endpoint::Endpoint, value_container::ValueContainer,
    },
};

use crate::{
    prelude::*,
    runtime::{
        Runtime, execution::execution_loop::state::RuntimeExecutionStack,
    },
};
pub use local::*;
use log::info;
pub use remote::*;
pub use script::*;

mod local;
mod remote;
mod script;

/// An execution context holds the persistent state for executing multiple scripts sequentially within the same context.
/// This can be either a local context, which is used for executing scripts in the same process, or a remote context,
/// which is used for executing scripts on a remote endpoint.
#[derive(Debug)]
pub enum ExecutionContext {
    Local(LocalExecutionContext),
    Remote(RemoteExecutionContext),
}
impl ExecutionContext {
    pub fn receivers(&self) -> Vec<Endpoint> {
        match self {
            ExecutionContext::Local(_) => {
                vec![Endpoint::LOCAL]
            }
            ExecutionContext::Remote(remote_context) => {
                remote_context.endpoints().to_vec()
            }
        }
    }
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

    fn runtime(&self) -> &Runtime {
        match self {
            ExecutionContext::Local(LocalExecutionContext {
                runtime, ..
            }) => runtime,
            ExecutionContext::Remote(RemoteExecutionContext {
                runtime,
                ..
            }) => runtime,
        }
    }

    /// Compiles a script using the compile scope of the execution context
    #[cfg(feature = "compiler")]
    pub fn compile(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
        receivers: Vec<Endpoint>,
    ) -> Result<DXBWithSharedValues, SpannedCompilerError> {
        use crate::compiler::compilation_result::CompilationResult;
        let compile_scope = self.compile_scope();
        // TODO #107: don't clone compile_scope if possible
        let res = compile_template(
            script,
            inserted_values
                .iter()
                .cloned()
                .map(Some)
                .collect::<Vec<_>>(),
            CompileOptions::new(compile_scope.clone(), receivers),
            self.runtime().clone(),
        );
        match res {
            Ok(CompilationResult {dxb_with_shared_values, scope, ..}) => {
                self.set_compile_scope(scope);
                Ok(dxb_with_shared_values)
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
                .join(" ")
        );

        #[cfg(feature = "decompiler")]
        {
            let decompiled =
                crate::decompiler::dxb_to_source_code::dxb_to_source_code(
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

    fn get_local_execution_input(
        &mut self,
        input: DXBWithSharedValues,
        initial_stack_values: Option<Vec<ValueContainer>>,
    ) -> Result<ExecutionInput, ExecutionError> {
        // show DXB and decompiled code if verbose is enabled
        if let ExecutionContext::Local(LocalExecutionContext {
            verbose: true,
            ..
        }) = &self
        {
            self.print_dxb_debug(&input.dxb)?;
        };

        match self {
            ExecutionContext::Remote(_) => {
                core::panic!("Remote execution requires a Runtime");
            }
            ExecutionContext::Local(LocalExecutionContext {
                runtime,
                loop_state,
                execution_options,
                verbose: _,
                caller_metadata,
                ..
            }) => {
                let input =
                    if let Some(initial_stack_values) = initial_stack_values {
                        // if we already have a loop state but also provided new initial stack values, this is an invalid state
                        if loop_state.is_some() {
                            return Err(ExecutionError::InvalidExecutionState);
                        }
                        ExecutionInput::new_with_stack(
                            input,
                            caller_metadata.clone(),
                            (*execution_options).clone(),
                            runtime.clone(),
                            RuntimeExecutionStack::new(initial_stack_values),
                        )
                    } else {
                        ExecutionInput::new_with_loop_state(
                            input,
                            caller_metadata.clone(),
                            (*execution_options).clone(),
                            runtime.clone(),
                            loop_state.take(),
                        )
                    };

                Ok(input)
            }
        }
    }

    /// Executes DXB in a local execution context.
    pub fn execute_dxb_sync(
        &mut self,
        dxb: DXBWithSharedValues,
        initial_stack_values: Option<Vec<ValueContainer>>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let execution_input =
            self.get_local_execution_input(dxb, initial_stack_values)?;
        let res = execute_dxb_sync(execution_input);
        self.intercept_intermediate_result(res)
    }

    /// Executes a script in a local execution context.
    #[cfg(feature = "compiler")]
    pub fn execute_sync(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        // TODO: insert values as stack values, don't compile?
        let dxb =
            self.compile(script, inserted_values, vec![Endpoint::LOCAL])?;
        self.execute_dxb_sync(dxb, None)
            .map_err(ScriptExecutionError::from)
    }

    pub async fn execute_dxb(
        &mut self,
        input: DXBWithSharedValues,
        initial_stack_values: Option<Vec<ValueContainer>>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match self {
            ExecutionContext::Local(..) => {
                let res = {
                    let execution_input = self.get_local_execution_input(
                        input,
                        initial_stack_values,
                    )?;
                    execute_dxb(execution_input).await
                };
                self.intercept_intermediate_result(res)
            }
            // remote execution is not supported directly in execution context
            ExecutionContext::Remote(..) => {
                core::panic!("Remote execution requires a Runtime");
            }
        }
    }

    /// Intercepts an intermediate execution result,
    /// storing the loop state if present in the execution context and returning the intermediate result as an Ok value.
    /// Note: this function assumes that self is a Local execution context
    fn intercept_intermediate_result(
        &mut self,
        execution_result: Result<Option<ValueContainer>, ExecutionError>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match execution_result {
            Err(ExecutionError::IntermediateResultWithState(
                intermediate_result,
                Some(state),
            )) => {
                match self {
                    ExecutionContext::Local(LocalExecutionContext {
                        loop_state,
                        ..
                    }) => {
                        loop_state.replace(state);
                        Ok(*intermediate_result)
                    }
                    _ => unreachable!(), // note: this must be ensured by the caller
                }
            }
            _ => execution_result,
        }
    }

    #[cfg(feature = "compiler")]
    pub async fn execute(
        &mut self,
        script: &str,
        inserted_values: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        // TODO: insert values as stack values, don't compile?
        let dxb =
            self.compile(script, inserted_values, vec![Endpoint::LOCAL])?;
        self.execute_dxb(dxb, None)
            .await
            .map_err(ScriptExecutionError::from)
    }

    /// Returns a memory dump of the current state of the execution context if available.
    pub fn memory_dump(&self) -> Option<MemoryDump> {
        match self {
            ExecutionContext::Local(_local_context) => {
                todo!("#650 Undescribed by author.")
                // Some(local_context.memory_dump())
            }
            // TODO #397: also support remote memory dump if possible
            ExecutionContext::Remote(_) => None,
        }
    }
}
