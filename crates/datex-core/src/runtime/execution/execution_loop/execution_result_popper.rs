use crate::{
    dxb_parser::instruction_collector::{
        CollectedResults, CollectionResultsPopper,
    },
    prelude::*,
    runtime::execution::{
        ExecutionError, InvalidProgramError,
        execution_loop::{
            collected_execution_result::CollectedExecutionResult,
            runtime_value::RuntimeValue, state::RuntimeExecutionState,
        },
    },
    types::{r#type::Type, type_definition::TypeDefinition},
    values::value_container::ValueContainer,
};

impl
    CollectionResultsPopper<
        CollectedExecutionResult,
        Option<RuntimeValue>,
        ValueContainer,
        ValueContainer,
        Type,
        TypeDefinition,
    > for CollectedResults<CollectedExecutionResult>
{
    fn try_extract_type_definition(
        result: CollectedExecutionResult,
    ) -> Option<TypeDefinition> {
        match result {
            CollectedExecutionResult::TypeDefinition(ty) => Some(*ty),
            CollectedExecutionResult::Type(ty) => {
                Some(ty.convert_to_definition())
            }
            _ => None,
        }
    }
    fn try_extract_value(
        result: CollectedExecutionResult,
    ) -> Option<Option<RuntimeValue>> {
        match result {
            CollectedExecutionResult::Value(val) => Some(*val),
            _ => None,
        }
    }

    fn try_extract_type(result: CollectedExecutionResult) -> Option<Type> {
        match result {
            CollectedExecutionResult::Type(ty) => Some(*ty),
            CollectedExecutionResult::TypeDefinition(definition) => {
                Some(definition.convert_to_type())
            }
            _ => None,
        }
    }

    fn try_extract_key_value_pair(
        result: CollectedExecutionResult,
    ) -> Option<(ValueContainer, ValueContainer)> {
        match result {
            CollectedExecutionResult::KeyValuePair(deref!((key, value))) => {
                Some((key, value))
            }
            _ => None,
        }
    }
}

impl CollectedResults<CollectedExecutionResult> {
    /// Collect multiple owned value containers
    pub fn try_collect_value_containers(
        mut self,
        state: &mut RuntimeExecutionState,
    ) -> Result<Vec<ValueContainer>, ExecutionError> {
        let count = self.len();
        let mut expressions = Vec::with_capacity(count);
        for _ in 0..count {
            expressions.push(self.try_pop_value_container(state)?);
        }
        expressions.reverse();
        Ok(expressions)
    }

    /// Collect multiple owned runtime values.
    pub fn try_collect_runtime_values(
        mut self,
    ) -> Result<Vec<RuntimeValue>, ExecutionError> {
        let count = self.len();
        let mut expressions = Vec::with_capacity(count);
        for _ in 0..count {
            expressions.push(self.try_pop_runtime_value()?);
        }
        expressions.reverse();
        Ok(expressions)
    }

    /// Pops a runtime value result, returning an error if none exists
    pub fn try_pop_runtime_value(
        &mut self,
    ) -> Result<RuntimeValue, ExecutionError> {
        self.pop_value().ok_or(ExecutionError::invalid_program(
            InvalidProgramError::ExpectedValue,
        ))
    }

    /// Pops an owned value container, returning an error if none exists.
    /// If the value is a slot address, it is resolved to a value container and the slot is removed from the runtime state.
    pub fn try_pop_value_container(
        &mut self,
        state: &mut RuntimeExecutionState,
    ) -> Result<ValueContainer, ExecutionError> {
        self.try_pop_runtime_value()?.into_value_container(state)
    }

    /// Pops a key-value pair result, returning an error if none exists
    pub fn try_collect_key_value_pair(
        mut self,
    ) -> Result<Vec<(ValueContainer, ValueContainer)>, ExecutionError> {
        let count = self.len();
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let (key, value) = self.pop_key_value_pair();
            pairs.push((key, value));
        }
        pairs.reverse();
        Ok(pairs)
    }
}
