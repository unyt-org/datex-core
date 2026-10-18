use core::fmt::Display;

use crate::{
    global::stack_index::StackIndex,
    prelude::*,
    runtime::Runtime,
    types::type_definition::callable::InvalidArgumentError,
    values::{
        core_values::callable::error::CallableError, value::Value,
        value_container::ValueContainer,
    },
};
use alloc::boxed::Box;

#[derive(Debug, PartialEq, Eq)]
pub enum ApplyError {
    UnsupportedApply,
    AsyncCallableRequiresAsyncExecution,
    CallableError(Box<CallableError>),
    InvalidArgumentError(Box<InvalidArgumentError>),
}
impl Display for ApplyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ApplyError::UnsupportedApply => {
                write!(f, "Value does not support apply operation")
            }
            ApplyError::CallableError(error) => {
                write!(f, "Error during callable application: {}", error)
            }
            ApplyError::AsyncCallableRequiresAsyncExecution => {
                write!(f, "Async callable requires async execution")
            }
            ApplyError::InvalidArgumentError(error) => {
                write!(f, "{}", error)
            }
        }
    }
}
impl From<CallableError> for ApplyError {
    fn from(error: CallableError) -> Self {
        ApplyError::CallableError(Box::new(error))
    }
}
impl From<InvalidArgumentError> for ApplyError {
    fn from(error: InvalidArgumentError) -> Self {
        ApplyError::InvalidArgumentError(Box::new(error))
    }
}

/// Represents an argument to be applied to a callable value.
#[derive(Debug)]
pub struct ApplyArgument {
    /// The value of the argument to be applied.
    pub value: ValueContainer,
    /// If the argument is passed as a local reference, the `passed_as_ref` field is set to true.
    pub passed_as_ref: bool,
}

impl ApplyArgument {
    pub fn referenced<T: Into<ValueContainer>>(value: T) -> Self {
        ApplyArgument {
            value: value.into(),
            passed_as_ref: true,
        }
    }
    pub fn owned<T: Into<ValueContainer>>(value: T) -> Self {
        ApplyArgument {
            value: value.into(),
            passed_as_ref: false,
        }
    }
}

impl From<ValueContainer> for ApplyArgument {
    fn from(value: ValueContainer) -> Self {
        ApplyArgument::owned(value)
    }
}

impl<T: Into<Value>> From<T> for ApplyArgument {
    fn from(value: T) -> Self {
        ApplyArgument {
            value: ValueContainer::Local(value.into()),
            passed_as_ref: false,
        }
    }
}

impl From<ApplyArgument> for ValueContainer {
    fn from(arg: ApplyArgument) -> Self {
        arg.value
    }
}

/// Returns a vector of ValueContainers for all arguments that were passed as local references.
pub fn get_borrowed_apply_argument_values(
    values: Vec<ApplyArgument>,
) -> Vec<ValueContainer> {
    values
        .into_iter()
        .filter(|arg| arg.passed_as_ref)
        .map(|arg| arg.value)
        .collect()
}

/// Returns a vector of ApplyArguments with the passed_as_ref field set based on the provided borrowed_args_stack_indices.
pub fn into_apply_arguments_with_stack_indices(
    values: Vec<ValueContainer>,
    borrowed_args_stack_indices: &[Option<StackIndex>],
) -> Vec<ApplyArgument> {
    values
        .into_iter()
        .enumerate()
        .map(|(i, v)| ApplyArgument {
            value: v,
            passed_as_ref: borrowed_args_stack_indices[i].is_some(),
        })
        .collect()
}

// TODO #351: return ApplyErrors including call stack information (or store call stack directly in ExecutionError)
pub trait Apply {
    /// Calls the [try_apply_sync] method and checks that the number of returned local references
    /// matches the number of arguments that were passed as local references.
    fn try_apply_sync_checked(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        let expected_ref_count =
            args.iter().filter(|arg| arg.passed_as_ref).count();
        let res = self.try_apply_sync(runtime, args)?;

        if res.1.len() != expected_ref_count {
            return Err(ApplyError::CallableError(Box::new(
                CallableError::LostBorrowedArguments {
                    expected: expected_ref_count,
                    actual: res.1.len(),
                },
            )));
        }

        Ok(res)
    }

    /// Calls the [try_apply_async] method and checks that the number of returned local references
    /// matches the number of arguments that were passed as local references.
    async fn try_apply_async_checked(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        let expected_ref_count =
            args.iter().filter(|arg| arg.passed_as_ref).count();
        let res = self.try_apply_async(runtime, args).await?;

        if res.1.len() != expected_ref_count {
            return Err(ApplyError::CallableError(Box::new(
                CallableError::LostBorrowedArguments {
                    expected: expected_ref_count,
                    actual: res.1.len(),
                },
            )));
        }

        Ok(res)
    }

    /// Applies multiple ValueContainer arguments to self
    /// Returns an Error if the value does not support sync apply.
    /// The return value is a tuple of the return value of the apply operation,
    /// and a vector containing all value containers for apply arguments that were passed as local references.
    /// Note: the count of the returned [Vec<ValueContainer>] corresponds to the count of arguments that have [passed_as_ref] set to [true].
    fn try_apply_sync(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError>;

    /// Applies multiple ValueContainer arguments to self on a sync or async callable.
    /// The return value is a tuple of the return value of the apply operation,
    /// and a vector containing all value containers for apply arguments that were passed as local references.
    /// Note: the count of the returned [Vec<ValueContainer>] corresponds to the count of arguments that have [passed_as_ref] set to [true].
    async fn try_apply_async(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError>;
}
