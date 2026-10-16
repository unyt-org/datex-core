use crate::{
    core_compiler::instruction_input::InstructionInput,
    instruction::{
        instruction_data::ApplyData, regular_instruction::RegularInstruction,
    },
    prelude::*,
    runtime::Runtime,
    shared_values::{SharedContainer, traits::SharedContainerCommon},
    traits::apply::{Apply, ApplyArgument, ApplyError},
    values::{
        core_values::callable::{Callable, error::CallableError},
        value_container::ValueContainer,
    },
};

impl Apply for SharedContainer {
    fn try_apply_sync(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        if !self.is_self_owned() {
            return Err(ApplyError::AsyncCallableRequiresAsyncExecution);
        }
        let base = self.base_shared_container();
        let callable = base
            .value_container()
            .try_as::<Callable>()
            .ok_or(ApplyError::UnsupportedApply)?;
        callable.try_apply_sync(runtime, args)
    }

    async fn try_apply_async(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        if !self.is_self_owned() {
            return self.apply_remote(runtime, args).await;
        }

        let callable = {
            let base = self.base_shared_container();
            let value = base
                .value_container()
                .try_as::<Callable>()
                .ok_or(ApplyError::UnsupportedApply)?;
            // Note value container is cloned here to prevent borrow of base_shared_container across await point.
            value.clone()
        };
        callable.try_apply_async(runtime, args).await
    }
}

impl SharedContainer {
    /// Calls the apply method on the owner endpoint of the shared value.
    async fn apply_remote(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        let mut instructions: Vec<InstructionInput> = vec![
            RegularInstruction::Apply(ApplyData {
                arg_count: args.len() as u8,
            })
            .into(),
        ];
        // append args
        instructions.extend(
            args.into_iter()
                .map(|v| InstructionInput::ValueContainer(v.value)),
        );
        // append the callee
        instructions.push(InstructionInput::ValueContainer(
            ValueContainer::Shared(self.clone()),
        ));

        // FIXME: restore borrowed stack values across remote execution.
        // For now, local borrows are not supported cross endpoint

        let res = runtime
            .execute_instructions_remote(
                vec![self.pointer_address().endpoint()],
                instructions,
            )
            .await
            .map_err(|e| {
                ApplyError::CallableError(Box::new(
                    CallableError::ExecutionError(e),
                ))
            })?;

        Ok((res, vec![]))
    }
}
