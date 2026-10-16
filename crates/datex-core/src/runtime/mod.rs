//! This module contains the implementation of the runtime, which is the backbone of the DATEX system.
use crate::{
    core_compiler::core_compilation_context::DXBWithSharedValues,
    runtime::execution::ExecutionError,
    values::value_container::ValueContainer,
};

use crate::prelude::*;
use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    result::Result,
};
use execution::context::{
    ExecutionContext, RemoteExecutionContext, ScriptExecutionError,
};
pub mod cache;
mod config;
pub mod execution;
mod incoming_sections;
mod internal;
mod logger;
pub mod pointer_address_provider;
pub mod pointer_availability_lookup;
pub mod remote_value_sync;
mod runner;
#[cfg(test)]
pub mod test_utils;

use crate::{
    core_compiler::instruction_input::InstructionInput,
    values::core_values::endpoint::Endpoint,
};
pub use config::*;
pub use internal::*;
pub use runner::*;

#[derive(Clone, Debug)]
pub struct Runtime {
    internal: Rc<RuntimeInternal>,
}

impl Deref for Runtime {
    type Target = RuntimeInternal;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

/// publicly exposed wrapper impl for the Runtime
/// around RuntimeInternal
impl Runtime {
    pub(crate) fn new(runtime_internal: RuntimeInternal) -> Runtime {
        Runtime {
            internal: Rc::new(runtime_internal),
        }
    }

    fn register_endpoint_property(&self, name: &str, value: ValueContainer) {
        self.internal
            .register_endpoint_property(name, value);
    }

    pub fn stub() -> Runtime {
        Runtime::new(RuntimeInternal::stub())
    }

    fn internal(&self) -> Rc<RuntimeInternal> {
        Rc::clone(&self.internal)
    }

    #[cfg(feature = "compiler")]
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

    #[cfg(feature = "compiler")]
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
        input: DXBWithSharedValues,
        initial_stack_values: Option<Vec<ValueContainer>>,
        execution_context: Option<&'a mut ExecutionContext>,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_dxb(
            self.internal(),
            input,
            initial_stack_values,
            execution_context,
            end_execution,
        )
        .await
    }

    pub fn execute_dxb_sync(
        &self,
        input: DXBWithSharedValues,
        initial_stack_values: Option<Vec<ValueContainer>>,
        execution_context: Option<&mut ExecutionContext>,
        end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_dxb_sync(
            self.internal(),
            input,
            initial_stack_values,
            execution_context,
            end_execution,
        )
    }

    pub async fn execute_remote(
        &self,
        remote_execution_context: &mut RemoteExecutionContext,
        input: DXBWithSharedValues,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_remote(
            self.internal(),
            remote_execution_context,
            input,
        )
        .await
    }

    pub async fn execute_instructions_remote(
        &self,
        endpoints: Vec<Endpoint>,
        instructions_input: Vec<InstructionInput>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeInternal::execute_instructions_remote(
            self.internal(),
            endpoints,
            instructions_input,
        )
        .await
    }
}
