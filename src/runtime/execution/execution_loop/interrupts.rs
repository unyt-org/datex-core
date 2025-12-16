use std::cell::RefCell;
use std::rc::Rc;
use crate::global::protocol_structures::instructions::{RawFullPointerAddress, RawInternalPointerAddress, RawLocalPointerAddress, RegularInstruction, TypeInstruction};
use crate::values::core_values::map::OwnedMapKey;
use crate::values::core_values::r#type::Type;
use crate::values::value_container::ValueContainer;

#[derive(Debug)]
pub enum ExecutionInterrupt {
    /// contains an optional ValueContainer that is intercepted by the consumer of a value or passed as the final result at the end of execution
    ValueReturn(Option<ValueContainer>),
    /// contains a Type that is intercepted by a consumer of a type value
    TypeReturn(Type),
    /// contains a key-value pair that is intercepted by a map construction operation
    KeyValuePairReturn((OwnedMapKey, ValueContainer)),
    /// indicates the end of an unbounded statements block - is intercepted by a statements block loop
    StatementsEnd(bool),
    AllocateSlot(u32, ValueContainer),
    GetSlotValue(u32),
    SetSlotValue(u32, ValueContainer),
    DropSlot(u32),
    // used for intermediate results in unbounded scopes
    SetActiveValue(Option<ValueContainer>),
    GetNextRegularInstruction,
    GetNextTypeInstruction,
    /// yields an external interrupt to be handled by the execution loop caller (for I/O operations, pointer resolution, remote execution, etc.)
    External(crate::runtime::execution::execution_loop::ExternalExecutionInterrupt)
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
    NextRegularInstruction(RegularInstruction),
    NextTypeInstruction(TypeInstruction),
}

#[derive(Debug, Clone)]
pub struct InterruptProvider {
    result: Rc<RefCell<Option<InterruptResult>>>
}

impl InterruptProvider {
    pub fn new() -> Self {
        Self {
            result: Rc::new(RefCell::new(None))
        }
    }

    pub fn provide_result(&self, result: InterruptResult) {
        *self.result.borrow_mut() = Some(result);
    }

    pub fn take_result(&self) -> Option<InterruptResult> {
        self.result.borrow_mut().take()
    }
}