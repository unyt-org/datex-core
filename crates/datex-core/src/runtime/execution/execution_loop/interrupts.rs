use core::cell::RefCell;

use crate::values::value_container::ValueContainer;

use crate::{
    core_compiler::core_compilation_context::DXBWithSharedValues,
    libs::core::core_lib_id::CoreLibId,
    prelude::*,
    shared_values::{
        ReferenceMutability, RemotePointerAddress, SelfOwnedPointerAddress,
    },
    traits::apply::ApplyArgument,
    values::core_values::endpoint::Endpoint,
};

#[derive(Debug)]
pub enum ExecutionInterrupt {
    // used for intermediate results in unbounded scopes
    SetActiveValue(Option<ValueContainer>),
    TakeActiveValue,
    /// yields an external interrupt to be handled by the execution loop caller (for I/O operations, pointer resolution, remote execution, etc.)
    External(ExternalExecutionInterrupt),
}

#[derive(Debug)]
pub enum ExternalExecutionInterrupt {
    Result(Option<ValueContainer>),
    GetReferenceToRemotePointer(RemotePointerAddress, ReferenceMutability),
    GetReferenceToLocalPointer(SelfOwnedPointerAddress),
    GetCoreLibValue(CoreLibId),
    RemoteExecution {
        input: DXBWithSharedValues,
        receivers: Vec<Endpoint>,
    },
    SetEndpointProperty {
        endpoint: Endpoint,
        property_name: String,
        value: ValueContainer,
    },
    GetEndpointProperty {
        endpoint: Endpoint,
        property_name: String,
    },
    Apply(ValueContainer, Vec<ApplyArgument>),
    CallMethod(ApplyArgument, String, Vec<ApplyArgument>),
}

#[derive(Debug)]
pub enum InterruptResult {
    /// Used to return a single resolved value or None
    ResolvedValue(Option<ValueContainer>),
    /// Used to return multiple resolved values
    ResolvedValues(Vec<ValueContainer>),
    /// Used in function calls: returns all borrowed arguments back, as well as an optional result value
    ResolvedValueAndBorrowedArgs((Option<ValueContainer>, Vec<ValueContainer>)),
}

#[derive(Debug, Clone)]
pub struct InterruptProvider {
    result: Rc<RefCell<Option<InterruptResult>>>,
}

impl PartialEq for InterruptProvider {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.result, &other.result)
    }
}

impl Eq for InterruptProvider {}

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
