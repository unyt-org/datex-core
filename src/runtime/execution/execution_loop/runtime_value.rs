use crate::stdlib::borrow::Cow;
use crate::runtime::execution::execution_loop::slots::{get_slot_value, get_slot_value_mut};
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::ExecutionError;
use crate::values::value_container::ValueContainer;

/// Represents a value in the runtime execution context, which can either be a direct
/// `ValueContainer` or a reference to a slot address where the value is stored.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    ValueContainer(ValueContainer),
    SlotAddress(u32)
}

impl From<ValueContainer> for RuntimeValue {
    fn from(value: ValueContainer) -> Self {
        RuntimeValue::ValueContainer(value)
    }
}

impl From<u32> for RuntimeValue {
    fn from(address: u32) -> Self {
        RuntimeValue::SlotAddress(address)
    }
}

impl RuntimeValue {
    /// Call the provided closure with a reference to the underlying `ValueContainer`.
    /// If the `RuntimeValue` is a slot address, it retrieves the value from the runtime state.
    pub fn with_mut_value_container<F, R>(&mut self, state: &mut RuntimeExecutionState, f: F) -> Result<R, ExecutionError>
    where
        F: FnOnce(&mut ValueContainer) -> R,
    {
        match self {
            RuntimeValue::ValueContainer(vc) => Ok(f(vc)),
            RuntimeValue::SlotAddress(addr) => {
                let slot_value = get_slot_value_mut(state, *addr)?;
                Ok(f(slot_value))
            }
        }
    }
    
    /// Creates an owned `ValueContainer` from the `RuntimeValue`.
    /// This possibly involves cloning the value if it is stored in a slot.
    /// Do not use this method if you want to work on the actual value without cloning it.
    pub fn into_cloned_value_container(self, state: &RuntimeExecutionState) -> Result<ValueContainer, ExecutionError> {
        match self {
            RuntimeValue::ValueContainer(vc) => Ok(vc),
            RuntimeValue::SlotAddress(addr) => {
                Ok(get_slot_value(state, addr)?.clone())
            }
        }
    }
}