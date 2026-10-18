use core::fmt::Display;

use crate::runtime::execution::ExecutionError;

#[derive(Debug, PartialEq, Eq)]
pub enum CallableError {
    InvalidSignature,
    RuntimeOnlyCallable,
    HiddenCallable,
    ExecutionError(ExecutionError),
    LostBorrowedArguments { expected: usize, actual: usize },
}
impl Display for CallableError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CallableError::InvalidSignature => {
                write!(f, "Invalid signature for callable")
            }
            CallableError::ExecutionError(error) => {
                write!(f, "Execution error: {}", error)
            }
            CallableError::RuntimeOnlyCallable => {
                write!(
                    f,
                    "This callable can only be called inside a runtime context"
                )
            }
            CallableError::HiddenCallable => {
                write!(
                    f,
                    "This callable is hidden and cannot be called directly"
                )
            }
            CallableError::LostBorrowedArguments { expected, actual } => {
                write!(
                    f,
                    "Lost borrowed arguments: expected {}, but got {}",
                    expected, actual
                )
            }
        }
    }
}

impl From<ExecutionError> for CallableError {
    fn from(error: ExecutionError) -> Self {
        CallableError::ExecutionError(error)
    }
}
