use crate::{
    prelude::*, shared_values::errors::AccessError, types::error::TypeError,
};
use core::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UpdateError {
    ImmutableValue,
    ImmutableReference,
    InvalidUpdate,
    AccessError(Box<AccessError>),
    TypeError(Box<TypeError>),
}
impl UpdateError {
    pub fn access_error<T: Into<AccessError>>(err: T) -> Self {
        UpdateError::AccessError(Box::new(err.into()))
    }
    pub fn type_error<T: Into<TypeError>>(err: T) -> Self {
        UpdateError::TypeError(Box::new(err.into()))
    }
}

impl From<AccessError> for UpdateError {
    fn from(err: AccessError) -> Self {
        UpdateError::AccessError(Box::new(err))
    }
}

impl From<TypeError> for UpdateError {
    fn from(err: TypeError) -> Self {
        UpdateError::TypeError(Box::new(err))
    }
}

impl Display for UpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UpdateError::InvalidUpdate => {
                core::write!(f, "Invalid update operation")
            }
            UpdateError::AccessError(e) => {
                core::write!(f, "Access error: {}", e)
            }
            UpdateError::TypeError(e) => {
                core::write!(f, "Type error: {}", e)
            }
            UpdateError::ImmutableValue => {
                core::write!(f, "Cannot update an immutable value")
            }
            UpdateError::ImmutableReference => {
                core::write!(f, "Cannot update an immutable reference")
            }
        }
    }
}
