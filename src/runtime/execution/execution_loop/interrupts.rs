use crate::global::protocol_structures::instructions::{
    RawFullPointerAddress, RawInternalPointerAddress, RawLocalPointerAddress,
    RegularInstruction, TypeInstruction,
};
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::values::core_values::map::MapKey;
use crate::values::core_values::r#type::Type;
use crate::values::value_container::ValueContainer;

#[derive(Debug)]
pub enum ExecutionInterrupt {
    AllocateSlot(u32, ValueContainer),
    GetSlotValue(u32),
    SetSlotValue(u32, ValueContainer),
    DropSlot(u32),
    // used for intermediate results in unbounded scopes
    SetActiveValue(Option<ValueContainer>),
    /// yields an external interrupt to be handled by the execution loop caller (for I/O operations, pointer resolution, remote execution, etc.)
    External(ExternalExecutionInterrupt),
}

#[derive(Debug)]
pub enum ExternalExecutionInterrupt {
    Result(Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveLocalPointer(RawLocalPointerAddress),
    ResolveInternalPointer(RawInternalPointerAddress),
    GetInternalSlotValue(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Apply(ValueContainer, Vec<ValueContainer>),
}

#[derive(Debug)]
pub enum InterruptResult {
    ResolvedValue(Option<ValueContainer>),
}

#[derive(Debug, Clone)]
pub struct InterruptProvider {
    result: Rc<RefCell<Option<InterruptResult>>>,
}

impl Default for InterruptProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl InterruptProvider {
    pub fn new() -> Self {
        Self {
            result: Rc::new(RefCell::new(None)),
        }
    }

    pub fn provide_result(&self, result: InterruptResult) {
        *self.result.borrow_mut() = Some(result);
    }

    pub fn take_result(&self) -> Option<InterruptResult> {
        self.result.borrow_mut().take()
    }
}
