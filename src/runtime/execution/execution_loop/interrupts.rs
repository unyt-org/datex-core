use crate::global::protocol_structures::instructions::{
    RawFullPointerAddress, RawInternalPointerAddress, RawLocalPointerAddress,
};
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::stdlib::vec::Vec;
use crate::values::core_values::map::MapKey;
use crate::values::value_container::{OwnedValueKey, ValueContainer};

#[derive(Debug)]
pub enum ExecutionInterrupt {
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
    RemoteExecution(ValueContainer, Vec<u8>),
    Apply(ValueContainer, Vec<ValueContainer>),
    SetProperty {
        target: ValueContainer, // TODO: move value containers by reference to allow modification on plain value containers (no cloning!)
        key: OwnedValueKey,
        value: ValueContainer,
    },
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
