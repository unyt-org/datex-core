//! The [CollectedExecutionResult] is a holder for a intermediate result from the execution loop.

use crate::{
    runtime::execution::execution_loop::runtime_value::RuntimeValue,
    types::{r#type::Type, type_definition::TypeDefinition},
    values::value_container::ValueContainer,
};

use crate::prelude::*;
#[derive(Debug)]
pub enum CollectedExecutionResult {
    /// contains an optional runtime value that is intercepted by the consumer of a value or passed as the final result at the end of execution
    Value(Box<Option<RuntimeValue>>),
    /// contains a [Type] that is intercepted by a consumer of a type value
    Type(Box<Type>),
    /// contains a [TypeDefinition] that is intercepted by a consumer of a type definition value
    TypeDefinition(Box<TypeDefinition>),
    /// contains a key-value pair that is intercepted by a map construction operation
    KeyValuePair(Box<(ValueContainer, ValueContainer)>),
}

impl CollectedExecutionResult {
    pub fn type_definition(definition: TypeDefinition) -> Self {
        CollectedExecutionResult::TypeDefinition(Box::new(definition))
    }
    pub fn type_value(ty: Type) -> Self {
        CollectedExecutionResult::Type(Box::new(ty))
    }
    pub fn key_value_pair(key: ValueContainer, value: ValueContainer) -> Self {
        CollectedExecutionResult::KeyValuePair(Box::new((key, value)))
    }
    pub fn value(value: Option<RuntimeValue>) -> Self {
        CollectedExecutionResult::Value(Box::new(value))
    }
}

impl From<Option<RuntimeValue>> for CollectedExecutionResult {
    fn from(value: Option<RuntimeValue>) -> Self {
        CollectedExecutionResult::value(value)
    }
}
impl From<ValueContainer> for CollectedExecutionResult {
    fn from(value: ValueContainer) -> Self {
        CollectedExecutionResult::value(Some(value.into()))
    }
}

impl From<RuntimeValue> for CollectedExecutionResult {
    fn from(value: RuntimeValue) -> Self {
        CollectedExecutionResult::value(Some(value))
    }
}
impl From<Type> for CollectedExecutionResult {
    fn from(value: Type) -> Self {
        CollectedExecutionResult::Type(Box::new(value))
    }
}

impl From<TypeDefinition> for CollectedExecutionResult {
    fn from(value: TypeDefinition) -> Self {
        CollectedExecutionResult::TypeDefinition(Box::new(value))
    }
}

impl From<(ValueContainer, ValueContainer)> for CollectedExecutionResult {
    fn from(value: (ValueContainer, ValueContainer)) -> Self {
        CollectedExecutionResult::KeyValuePair(Box::new(value))
    }
}
