use crate::dxb_parser::body::DXBParserError;
use crate::network::com_hub::ResponseError;
use crate::references::reference::{AccessError, AssignmentError, ReferenceCreationError};
use crate::runtime::execution::execution_loop::state::ExecutionLoopState;
use crate::stdlib::string::String;
use crate::types::error::IllegalTypeError;
use crate::values::value_container::{ValueContainer, ValueError};
use core::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidProgramError {
    // any unterminated sequence, e.g. missing key in key-value pair
    UnterminatedSequence,
    MissingRemoteExecutionReceiver,
    ExpectedTypeValue,
    ExpectedValue,
    ExpectedInstruction,
    ExpectedRegularInstruction,
    ExpectedTypeInstruction,
}

impl Display for InvalidProgramError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
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
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    DXBParserError(DXBParserError),
    ValueError(ValueError),
    InvalidProgram(InvalidProgramError),
    AccessError(AccessError),
    Unknown,
    NotImplemented(String),
    SlotNotAllocated(u32),
    SlotNotInitialized(u32),
    RequiresAsyncExecution,
    RequiresRuntime,
    ResponseError(ResponseError),
    IllegalTypeError(IllegalTypeError),
    ReferenceNotFound,
    DerefOfNonReference,
    InvalidTypeCast,
    ExpectedTypeValue,
    AssignmentError(AssignmentError),
    ReferenceFromValueContainerError(ReferenceCreationError),
    IntermediateResultWithState(
        Option<ValueContainer>,
        Option<ExecutionLoopState>,
    ),
}
impl From<ReferenceCreationError> for ExecutionError {
    fn from(error: ReferenceCreationError) -> Self {
        ExecutionError::ReferenceFromValueContainerError(error)
    }
}

impl From<AccessError> for ExecutionError {
    fn from(error: AccessError) -> Self {
        ExecutionError::AccessError(error)
    }
}

impl From<DXBParserError> for ExecutionError {
    fn from(error: DXBParserError) -> Self {
        ExecutionError::DXBParserError(error)
    }
}

impl From<ValueError> for ExecutionError {
    fn from(error: ValueError) -> Self {
        ExecutionError::ValueError(error)
    }
}

impl From<IllegalTypeError> for ExecutionError {
    fn from(error: IllegalTypeError) -> Self {
        ExecutionError::IllegalTypeError(error)
    }
}

impl From<InvalidProgramError> for ExecutionError {
    fn from(error: InvalidProgramError) -> Self {
        ExecutionError::InvalidProgram(error)
    }
}

impl From<ResponseError> for ExecutionError {
    fn from(error: ResponseError) -> Self {
        ExecutionError::ResponseError(error)
    }
}

impl From<AssignmentError> for ExecutionError {
    fn from(error: AssignmentError) -> Self {
        ExecutionError::AssignmentError(error)
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExecutionError::ReferenceFromValueContainerError(err) => {
                core::write!(f, "Reference from value container error: {err}")
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
            ExecutionError::ValueError(err) => {
                core::write!(f, "Value error: {err}")
            }
            ExecutionError::InvalidProgram(err) => {
                core::write!(f, "Invalid program error: {err}")
            }
            ExecutionError::NotImplemented(msg) => {
                core::write!(f, "Not implemented: {msg}")
            }
            ExecutionError::SlotNotAllocated(address) => {
                core::write!(
                    f,
                    "Tried to access unallocated slot at address {address}"
                )
            }
            ExecutionError::SlotNotInitialized(address) => {
                core::write!(
                    f,
                    "Tried to access uninitialized slot at address {address}"
                )
            }
            ExecutionError::RequiresAsyncExecution => {
                core::write!(f, "Program must be executed asynchronously")
            }
            ExecutionError::RequiresRuntime => {
                core::write!(f, "Execution requires a runtime to be set")
            }
            ExecutionError::ResponseError(err) => {
                core::write!(f, "Response error: {err}")
            }
            ExecutionError::IllegalTypeError(err) => {
                core::write!(f, "Illegal type: {err}")
            }
            ExecutionError::DerefOfNonReference => {
                core::write!(f, "Tried to dereference a non-reference value")
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
            ExecutionError::AccessError(err) => {
                core::write!(f, "Access error: {err}")
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
        }
    }
}
