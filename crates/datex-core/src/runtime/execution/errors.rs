use crate::{
    dxb_parser::body::DXBParserError,
    global::stack_index::StackIndex,
    libs::core::core_lib_id::CoreLibIdIndex,
    network::com_hub::network_response::ResponseError,
    prelude::*,
    runtime::{
        cache::shared_values_cache::CacheValueRetrievalError,
        execution::execution_loop::state::ExecutionLoopState,
        remote_value_sync::SubscriberError,
    },
    shared_values::errors::{
        AccessError, AssignmentError, SharedValueCreationError,
    },
    traits::apply::ApplyError,
    types::error::IllegalTypeError,
    value_updates::errors::UpdateError,
    values::value_container::{ValueContainer, error::ValueError},
};
use core::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidProgramError {
    // any unterminated sequence, e.g. missing key in key-value pair
    UnterminatedSequence,
    MissingRemoteExecutionReceiver,
    ExpectedTypeValue,
    ExpectedAliasType,
    ExpectedValue,
    ExpectedList,
    ExpectedInstruction,
    ExpectedRegularInstruction,
    ExpectedTypeInstruction,
    InvalidCoreLibId(CoreLibIdIndex),
    InvalidInstructionFormat,
    InvalidType,
}

impl Display for InvalidProgramError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InvalidProgramError::ExpectedAliasType => {
                core::write!(f, "Expected an alias type definition")
            }
            InvalidProgramError::UnterminatedSequence => {
                core::write!(f, "Unterminated sequence")
            }
            InvalidProgramError::MissingRemoteExecutionReceiver => {
                core::write!(f, "Missing remote execution receiver")
            }
            InvalidProgramError::ExpectedTypeValue => {
                core::write!(f, "Expected a type value")
            }
            InvalidProgramError::ExpectedValue => {
                core::write!(f, "Expected a value")
            }
            InvalidProgramError::ExpectedRegularInstruction => {
                core::write!(f, "Expected a regular instruction")
            }
            InvalidProgramError::ExpectedTypeInstruction => {
                core::write!(f, "Expected a type instruction")
            }
            InvalidProgramError::ExpectedInstruction => {
                core::write!(f, "Expected an instruction")
            }
            InvalidProgramError::ExpectedList => {
                core::write!(f, "Expected a list")
            }
            InvalidProgramError::InvalidCoreLibId(core_lib_id) => {
                core::write!(f, "Invalid core library id: {core_lib_id}")
            }
            InvalidProgramError::InvalidInstructionFormat => {
                core::write!(f, "Invalid instruction format")
            }
            InvalidProgramError::InvalidType => {
                core::write!(f, "Invalid type")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionError {
    Unspecified(String), // TODO: replace with nested stack trace exceptions
    DXBParserError(Box<DXBParserError>),
    ValueError(Box<ValueError>),
    InvalidProgram(Box<InvalidProgramError>),
    AccessError(Box<AccessError>),
    MethodNotFound(String),
    CacheValueRetrievalError(Box<CacheValueRetrievalError>),
    UpdateError(Box<UpdateError>),
    SubscriberError(Box<SubscriberError>),
    Unknown,
    InvalidExecutionState,
    NotImplemented(String),
    StackValueNotAllocated(StackIndex),
    StackOutOfBoundsAccess(StackIndex),
    InternalSlotDoesNotExist(u32),
    RequiresAsyncExecution,
    ResponseError(Box<ResponseError>),
    IllegalTypeError(Box<IllegalTypeError>),
    ReferenceNotFound,
    InvalidUnbox,
    InvalidTypeCast,
    ExpectedTypeValue,
    InvalidSharedValueType,
    ExpectedSharedValue,
    ExpectedLocalValue,
    ExpectedEntityValue,
    ExpectedOwnedSharedValue,
    MutableReferenceToNonMutableValue,
    AssignmentError(Box<AssignmentError>),
    ReferenceCreationError(Box<SharedValueCreationError>),
    IntermediateResultWithState(
        Box<Option<ValueContainer>>,
        Box<Option<ExecutionLoopState>>,
    ),
    ApplyError(Box<ApplyError>),
    UnauthorizedMove,
    InvalidMove,
    MoveToMultipleEndpoints,
    UnclonableValue, // happens for native rust structs that don't implement clone. FIXME: this should not happen in execution loop
    StackRestoreMismatch,
}
impl ExecutionError {
    pub fn intermediate_result_with_state(
        value_opt: Option<ValueContainer>,
        state_opt: Option<ExecutionLoopState>,
    ) -> Self {
        ExecutionError::IntermediateResultWithState(
            Box::new(value_opt),
            Box::new(state_opt),
        )
    }
    pub fn invalid_program(error: InvalidProgramError) -> Self {
        ExecutionError::InvalidProgram(Box::new(error))
    }
    pub fn value_error(error: ValueError) -> Self {
        ExecutionError::ValueError(Box::new(error))
    }
    pub fn access_error(error: AccessError) -> Self {
        ExecutionError::AccessError(Box::new(error))
    }
    pub fn update_error(error: UpdateError) -> Self {
        ExecutionError::UpdateError(Box::new(error))
    }
    pub fn cache_value_retrieval_error(
        error: CacheValueRetrievalError,
    ) -> Self {
        ExecutionError::CacheValueRetrievalError(Box::new(error))
    }
    pub fn subscriber_error(error: SubscriberError) -> Self {
        ExecutionError::SubscriberError(Box::new(error))
    }
    pub fn response_error(error: ResponseError) -> Self {
        ExecutionError::ResponseError(Box::new(error))
    }
    pub fn illegal_type_error(error: IllegalTypeError) -> Self {
        ExecutionError::IllegalTypeError(Box::new(error))
    }
    pub fn apply_error(error: ApplyError) -> Self {
        ExecutionError::ApplyError(Box::new(error))
    }
    pub fn reference_creation_error(error: SharedValueCreationError) -> Self {
        ExecutionError::ReferenceCreationError(Box::new(error))
    }
    pub fn assignment_error(error: AssignmentError) -> Self {
        ExecutionError::AssignmentError(Box::new(error))
    }
    pub fn dxb_parser_error(error: DXBParserError) -> Self {
        ExecutionError::DXBParserError(Box::new(error))
    }
}

impl From<ApplyError> for ExecutionError {
    fn from(error: ApplyError) -> Self {
        ExecutionError::ApplyError(Box::new(error))
    }
}

impl From<SharedValueCreationError> for ExecutionError {
    fn from(error: SharedValueCreationError) -> Self {
        ExecutionError::ReferenceCreationError(Box::new(error))
    }
}

impl From<CacheValueRetrievalError> for ExecutionError {
    fn from(err: CacheValueRetrievalError) -> Self {
        ExecutionError::CacheValueRetrievalError(Box::new(err))
    }
}

impl From<AccessError> for ExecutionError {
    fn from(error: AccessError) -> Self {
        ExecutionError::AccessError(Box::new(error))
    }
}

impl From<UpdateError> for ExecutionError {
    fn from(error: UpdateError) -> Self {
        ExecutionError::UpdateError(Box::new(error))
    }
}

impl From<DXBParserError> for ExecutionError {
    fn from(error: DXBParserError) -> Self {
        ExecutionError::DXBParserError(Box::new(error))
    }
}

impl From<ValueError> for ExecutionError {
    fn from(error: ValueError) -> Self {
        ExecutionError::ValueError(Box::new(error))
    }
}

impl From<IllegalTypeError> for ExecutionError {
    fn from(error: IllegalTypeError) -> Self {
        ExecutionError::IllegalTypeError(Box::new(error))
    }
}

impl From<InvalidProgramError> for ExecutionError {
    fn from(error: InvalidProgramError) -> Self {
        ExecutionError::InvalidProgram(Box::new(error))
    }
}

impl From<ResponseError> for ExecutionError {
    fn from(error: ResponseError) -> Self {
        ExecutionError::ResponseError(Box::new(error))
    }
}

impl From<AssignmentError> for ExecutionError {
    fn from(error: AssignmentError) -> Self {
        ExecutionError::AssignmentError(Box::new(error))
    }
}

impl From<SubscriberError> for ExecutionError {
    fn from(error: SubscriberError) -> Self {
        ExecutionError::SubscriberError(Box::new(error))
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExecutionError::ReferenceCreationError(err) => {
                core::write!(f, "Reference from value container error: {err}")
            }
            ExecutionError::ApplyError(err) => {
                core::write!(f, "Apply error: {err}")
            }
            ExecutionError::ReferenceNotFound => {
                core::write!(f, "Reference not found")
            }
            ExecutionError::DXBParserError(err) => {
                core::write!(f, "Parser error: {err}")
            }
            ExecutionError::Unknown => {
                core::write!(f, "Unknown execution error")
            }
            ExecutionError::InvalidExecutionState => {
                core::write!(f, "Invalid execution state")
            }
            ExecutionError::ValueError(err) => {
                core::write!(f, "Value error: {err}")
            }
            ExecutionError::SubscriberError(err) => {
                core::write!(f, "Subscriber error: {err}")
            }
            ExecutionError::InvalidProgram(err) => {
                core::write!(f, "Invalid program error: {err}")
            }
            ExecutionError::NotImplemented(msg) => {
                core::write!(f, "Not implemented: {msg}")
            }
            ExecutionError::RequiresAsyncExecution => {
                core::write!(f, "Program must be executed asynchronously")
            }
            ExecutionError::ResponseError(err) => {
                core::write!(f, "Response error: {err}")
            }
            ExecutionError::IllegalTypeError(err) => {
                core::write!(f, "Illegal type: {err}")
            }
            ExecutionError::InvalidUnbox => {
                core::write!(f, "Tried to unbox a non-reference value")
            }
            ExecutionError::AssignmentError(err) => {
                core::write!(f, "Assignment error: {err}")
            }
            ExecutionError::InvalidTypeCast => {
                core::write!(f, "Invalid type cast")
            }
            ExecutionError::ExpectedTypeValue => {
                core::write!(f, "Expected a type value")
            }
            ExecutionError::InvalidSharedValueType => {
                core::write!(f, "Invalid shared value type")
            }
            ExecutionError::AccessError(err) => {
                core::write!(f, "Access error: {err}")
            }
            ExecutionError::MethodNotFound(method_name) => {
                core::write!(f, "Method not found: {method_name}")
            }
            ExecutionError::CacheValueRetrievalError(err) => {
                core::write!(f, "Cache value retrieval error: {err}")
            }
            ExecutionError::IntermediateResultWithState(
                value_opt,
                state_opt,
            ) => {
                core::write!(
                    f,
                    "Execution produced an intermediate result: {:?} with state: {:?}",
                    value_opt,
                    state_opt
                )
            }
            ExecutionError::UnauthorizedMove => {
                core::write!(f, "Unauthorized move of shared pointer")
            }
            ExecutionError::InvalidMove => {
                core::write!(f, "Invalid move of shared pointer")
            }
            ExecutionError::MoveToMultipleEndpoints => {
                core::write!(f, "Illegal move to multiple endpoints")
            }
            ExecutionError::UnclonableValue => {
                core::write!(f, "Tried to clone an unclonable value")
            }
            ExecutionError::ExpectedSharedValue => {
                core::write!(
                    f,
                    "Expected a shared value, but got a local value"
                )
            }
            ExecutionError::ExpectedLocalValue => {
                core::write!(
                    f,
                    "Expected a local value, but got a shared value"
                )
            }
            ExecutionError::ExpectedEntityValue => {
                core::write!(
                    f,
                    "Expected an entity value, but got a non-entity value"
                )
            }
            ExecutionError::ExpectedOwnedSharedValue => {
                core::write!(
                    f,
                    "Expected an owned shared value, but got a non-owned shared value"
                )
            }
            ExecutionError::MutableReferenceToNonMutableValue => {
                core::write!(
                    f,
                    "Tried to create a mutable reference to a non-mutable value"
                )
            }
            ExecutionError::StackValueNotAllocated(index) => {
                core::write!(
                    f,
                    "Tried to access unallocated stack value at index {index}"
                )
            }
            ExecutionError::StackOutOfBoundsAccess(index) => {
                core::write!(
                    f,
                    "Tried to access out of bounds stack value at index {index}"
                )
            }
            ExecutionError::InternalSlotDoesNotExist(index) => {
                core::write!(f, "Internal slot does not exist at index {index}")
            }
            ExecutionError::UpdateError(err) => {
                core::write!(f, "Value update error: {err}")
            }
            ExecutionError::Unspecified(msg) => {
                core::write!(f, "Unspecified error: {msg}")
            }
            ExecutionError::StackRestoreMismatch => {
                core::write!(f, "Stack restore mismatch")
            }
        }
    }
}
