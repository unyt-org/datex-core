use crate::shared_values::SharedContainerOwnership;
use core::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnexpectedSharedContainerOwnershipError {
    pub expected: SharedContainerOwnership,
    pub actual: SharedContainerOwnership,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnexpectedImmutableSharedContainerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnexpectedImmutableReferenceError;

impl Display for UnexpectedSharedContainerOwnershipError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Unexpected shared container ownership: expected {:?}, actual {:?}",
            self.expected, self.actual
        )
    }
}

impl Display for UnexpectedImmutableSharedContainerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Unexpected immutable shared container")
    }
}

impl Display for UnexpectedImmutableReferenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Unexpected immutable reference")
    }
}

use crate::{
    prelude::*,
    types::error::TypeError,
    values::{
        core_values::map::MapAccessError, value_container::ValueContainer,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SharedValueCreationError {
    InvalidType,
    MutabilityMismatch,
}

impl Display for SharedValueCreationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SharedValueCreationError::InvalidType => {
                write!(
                    f,
                    "Cannot create shared value from value container: invalid type"
                )
            }
            SharedValueCreationError::MutabilityMismatch => {
                write!(
                    f,
                    "Cannot create mutable shared value for immutable value"
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexOutOfBoundsError {
    pub index: u32,
}

impl Display for IndexOutOfBoundsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Index out of bounds: {}", self.index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyNotFoundError {
    pub key: Box<ValueContainer>,
}
impl KeyNotFoundError {
    pub fn new(key: ValueContainer) -> Self {
        KeyNotFoundError { key: Box::new(key) }
    }
}

impl Display for KeyNotFoundError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Property not found: {}", self.key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessError {
    InvalidOperation(String),
    KeyNotFound(KeyNotFoundError),
    IndexOutOfBounds(IndexOutOfBoundsError),
    MapAccessError(MapAccessError),
    InvalidIndexKey,
}

impl From<IndexOutOfBoundsError> for AccessError {
    fn from(err: IndexOutOfBoundsError) -> Self {
        AccessError::IndexOutOfBounds(err)
    }
}

impl From<MapAccessError> for AccessError {
    fn from(err: MapAccessError) -> Self {
        AccessError::MapAccessError(err)
    }
}

impl From<KeyNotFoundError> for AccessError {
    fn from(err: KeyNotFoundError) -> Self {
        AccessError::KeyNotFound(err)
    }
}

impl Display for AccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AccessError::MapAccessError(err) => {
                write!(f, "Map access error: {}", err)
            }
            AccessError::InvalidOperation(op) => {
                write!(f, "Invalid operation: {}", op)
            }
            AccessError::KeyNotFound(key) => {
                write!(f, "{}", key)
            }
            AccessError::IndexOutOfBounds(error) => {
                write!(f, "{}", error)
            }
            AccessError::InvalidIndexKey => {
                write!(f, "Invalid index key")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AssignmentError {
    ImmutableReference,
    TypeError(Box<TypeError>),
}

impl Display for AssignmentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AssignmentError::ImmutableReference => {
                write!(f, "Cannot assign to an immutable reference")
            }
            AssignmentError::TypeError(e) => {
                write!(f, "Type error: {}", e)
            }
        }
    }
}
