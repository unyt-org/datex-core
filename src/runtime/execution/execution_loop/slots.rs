use crate::global::slots::InternalSlot;
use crate::runtime::execution::ExecutionError;
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::values::value_container::ValueContainer;
use num_enum::TryFromPrimitive;

pub fn get_slot_value_mut(
    runtime_state: &mut RuntimeExecutionState,
    address: u32,
) -> Result<&mut ValueContainer, ExecutionError> {
    runtime_state.slots.get_slot_value_mut(address)
}

pub fn get_slot_value(
    runtime_state: &RuntimeExecutionState,
    address: u32,
) -> Result<&ValueContainer, ExecutionError> {
    runtime_state.slots.get_slot_value(address)
}

pub fn get_internal_slot_value(
    runtime_state: &RuntimeExecutionState,
    slot: u32,
) -> Result<ValueContainer, ExecutionError> {
    if let Some(runtime) = &runtime_state.runtime_internal {
        // convert slot to InternalSlot enum
        let slot = InternalSlot::try_from_primitive(slot)
            .map_err(|_| ExecutionError::SlotNotAllocated(slot))?;
        let res = match slot {
            InternalSlot::ENDPOINT => {
                ValueContainer::from(runtime.endpoint.clone())
            }
        };
        Ok(res)
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}
