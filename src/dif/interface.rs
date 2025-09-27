use std::fmt::Display;

use crate::dif::DIFUpdate;
use crate::dif::r#type::DIFTypeContainer;
use crate::dif::value::DIFValueContainer;
use crate::references::observers::{ObserverError, ReferenceObserver};
use crate::references::reference::{
    AccessError, AssignmentError, ReferenceFromValueContainerError,
    ReferenceMutability, TypeError,
};
use crate::runtime::execution::ExecutionError;
use crate::values::pointer::PointerAddress;

#[derive(Debug)]
pub enum DIFObserveError {
    ReferenceNotFound,
    ObserveError(ObserverError),
}
impl From<ObserverError> for DIFObserveError {
    fn from(err: ObserverError) -> Self {
        DIFObserveError::ObserveError(err)
    }
}
impl Display for DIFObserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DIFObserveError::ReferenceNotFound => {
                write!(f, "Reference not found")
            }
            DIFObserveError::ObserveError(e) => {
                write!(f, "Observe error: {}", e)
            }
        }
    }
}

#[derive(Debug)]
pub enum DIFUpdateError {
    ReferenceNotFound,
    InvalidUpdate,
    AccessError(AccessError),
    AssignmentError(AssignmentError),
    TypeError(TypeError),
}
impl Display for DIFUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DIFUpdateError::ReferenceNotFound => {
                write!(f, "Reference not found")
            }
            DIFUpdateError::InvalidUpdate => {
                write!(f, "Invalid update operation")
            }
            DIFUpdateError::AccessError(e) => write!(f, "Access error: {}", e),
            DIFUpdateError::AssignmentError(e) => {
                write!(f, "Assignment error: {}", e)
            }
            DIFUpdateError::TypeError(e) => write!(f, "Type error: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum DIFApplyError {
    ExecutionError(ExecutionError),
    ReferenceNotFound,
}
impl Display for DIFApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DIFApplyError::ExecutionError(e) => {
                write!(f, "Execution error: {}", e)
            }
            DIFApplyError::ReferenceNotFound => {
                write!(f, "Reference not found")
            }
        }
    }
}

#[derive(Debug)]
pub enum DIFCreatePointerError {
    ReferenceNotFound,
    ReferenceFromValueContainerError(ReferenceFromValueContainerError),
}

impl Display for DIFCreatePointerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DIFCreatePointerError::ReferenceNotFound => {
                write!(f, "Reference not found")
            }
            DIFCreatePointerError::ReferenceFromValueContainerError(e) => {
                write!(f, "Reference from value container error: {}", e)
            }
        }
    }
}

#[derive(Debug)]
pub enum DIFResolveReferenceError {
    ReferenceNotFound,
}
impl Display for DIFResolveReferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DIFResolveReferenceError::ReferenceNotFound => {
                write!(f, "Reference not found")
            }
        }
    }
}

impl From<ReferenceFromValueContainerError> for DIFCreatePointerError {
    fn from(err: ReferenceFromValueContainerError) -> Self {
        DIFCreatePointerError::ReferenceFromValueContainerError(err)
    }
}

pub trait DIFInterface {
    /// Applies a DIF update to the value at the given pointer address.
    fn update(
        &mut self,
        address: PointerAddress,
        update: DIFUpdate,
    ) -> Result<(), DIFUpdateError>;

    /// Executes an apply operation, applying the `value` to the `callee`.
    fn apply(
        &mut self,
        callee: DIFValueContainer,
        value: DIFValueContainer,
    ) -> Result<DIFValueContainer, DIFApplyError>;

    /// Creates a new pointer with the given DIF value and returns its address.
    async fn create_pointer(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeContainer>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError>;

    fn create_pointer_sync(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeContainer>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError>;

    async fn resolve_pointer_address_external(
        &self,
        address: PointerAddress,
    ) -> Result<DIFValueContainer, DIFResolveReferenceError>;

    fn resolve_pointer_address_in_memory(
        &self,
        address: PointerAddress,
    ) -> Result<DIFValueContainer, DIFResolveReferenceError>;

    /// Starts observing changes to the pointer at the given address.
    /// As long as the pointer is observed, it will not be garbage collected.
    fn observe_pointer(
        &self,
        address: PointerAddress,
        observer: ReferenceObserver,
    ) -> Result<u32, DIFObserveError>;

    /// Stops observing changes to the pointer at the given address.
    /// If no other references to the pointer exist, it may be garbage collected after this call.
    fn unobserve_pointer(
        &self,
        address: PointerAddress,
        observer_id: u32,
    ) -> Result<(), DIFObserveError>;
}
