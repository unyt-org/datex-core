use crate::collections::HashMap;
use crate::references::observers::TransceiverId;
use crate::runtime::RuntimeInternal;
use crate::runtime::execution::ExecutionError;
use crate::runtime::execution::execution_loop::ExternalExecutionInterrupt;
use crate::runtime::execution::execution_loop::interrupts::InterruptProvider;
use crate::stdlib::boxed::Box;
use crate::stdlib::rc::Rc;
use crate::stdlib::vec::Vec;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;
use core::fmt::Debug;

pub struct ExecutionLoopState {
    pub iterator: Box<
        dyn Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>>,
    >,
    pub dxb_body: Rc<RefCell<Vec<u8>>>,
    pub(crate) interrupt_provider: InterruptProvider,
}

impl Debug for ExecutionLoopState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExecutionIterator")
            .field("dxb_body_length", &self.dxb_body.borrow().len())
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct RuntimeExecutionState {
    /// Local memory slots for current execution context.
    /// TODO: replace this with a local stack and deprecate local slots?
    pub(crate) slots: RuntimeExecutionSlots,
    pub(crate) runtime_internal: Option<Rc<RuntimeInternal>>,
    pub(crate) source_id: TransceiverId,
}

#[derive(Debug, Default)]
pub struct RuntimeExecutionSlots {
    pub(crate) slots: HashMap<u32, Option<ValueContainer>>,
}

impl RuntimeExecutionSlots {
    /// Allocates a new slot with the given slot address.
    pub(crate) fn allocate_slot(
        &mut self,
        address: u32,
        value: Option<ValueContainer>,
    ) {
        self.slots.insert(address, value);
    }

    /// Drops a slot by its address, returning the value if it existed.
    /// If the slot is not allocated, it returns an error.
    pub(crate) fn drop_slot(
        &mut self,
        address: u32,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .remove(&address)
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }

    /// Sets the value of a slot, returning the previous value if it existed.
    /// If the slot is not allocated, it returns an error.
    pub(crate) fn set_slot_value(
        &mut self,
        address: u32,
        value: ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .insert(address, Some(value))
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }

    /// Retrieves a reference to the value of a slot by its address.
    /// If the slot is not allocated, it returns an error.
    pub(crate) fn get_slot_value(
        &self,
        address: u32,
    ) -> Result<&ValueContainer, ExecutionError> {
        self.slots
            .get(&address)
            .and_then(|inner| inner.as_ref())
            .ok_or_else(|| ExecutionError::SlotNotAllocated(address))
    }

    /// Retrieves a mutable reference to the value of a slot by its address.
    /// If the slot is not allocated, it returns an error.
    pub(crate) fn get_slot_value_mut(
        &mut self,
        address: u32,
    ) -> Result<&mut ValueContainer, ExecutionError> {
        self.slots
            .get_mut(&address)
            .and_then(|inner| inner.as_mut())
            .ok_or_else(|| ExecutionError::SlotNotAllocated(address))
    }
}
