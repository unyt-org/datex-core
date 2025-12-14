use core::cell::RefCell;
use std::fmt::Debug;
use crate::parser::next_instructions_stack::NextInstructionsStack;
use crate::runtime::execution::execution_loop::ExternalExecutionInterrupt;
use crate::stdlib::collections::HashMap;
use crate::stdlib::rc::Rc;
use crate::runtime::execution::ExecutionError;
use crate::runtime::RuntimeInternal;
use crate::values::value_container::ValueContainer;

pub struct ExecutionLoopState {
    pub iterator: Box<dyn Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>>>,
    pub dxb_body: Rc<RefCell<Vec<u8>>>,
}

impl Debug for ExecutionLoopState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionIterator")
            .field("dxb_body_length", &self.dxb_body.borrow().len())
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct RuntimeExecutionState {
    pub(crate) slots: RefCell<HashMap<u32, Option<ValueContainer>>>,
    /// Used to track the next instructions to be executed, distinguishing between regular and type instructions.
    pub(crate) next_instructions_stack: NextInstructionsStack,
    runtime_internal: Option<Rc<RuntimeInternal>>,
}

impl RuntimeExecutionState {
    pub fn new(runtime_internal: Rc<RuntimeInternal>) -> Self {
        Self {
            runtime_internal: Some(runtime_internal),
            ..Default::default()
        }
    }

    pub fn runtime_internal(&self) -> &Option<Rc<RuntimeInternal>> {
        &self.runtime_internal
    }

    pub fn set_runtime_internal(
        &mut self,
        runtime_internal: Rc<RuntimeInternal>,
    ) {
        self.runtime_internal = Some(runtime_internal);
    }

    /// Allocates a new slot with the given slot address.
    pub(crate) fn allocate_slot(&self, address: u32, value: Option<ValueContainer>) {
        self.slots.borrow_mut().insert(address, value);
    }

    /// Drops a slot by its address, returning the value if it existed.
    /// If the slot is not allocated, it returns an error.
    pub(crate) fn drop_slot(
        &self,
        address: u32,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .borrow_mut()
            .remove(&address)
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }

    /// Sets the value of a slot, returning the previous value if it existed.
    /// If the slot is not allocated, it returns an error.
    pub(crate) fn set_slot_value(
        &self,
        address: u32,
        value: ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .borrow_mut()
            .insert(address, Some(value))
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }

    /// Retrieves the value of a slot by its address.
    /// If the slot is not allocated, it returns an error.
    pub(crate) fn get_slot_value(
        &self,
        address: u32,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .borrow_mut()
            .get(&address)
            .cloned()
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }
}