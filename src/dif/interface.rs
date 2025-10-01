use crate::dif::r#type::DIFTypeContainer;
use crate::dif::update::DIFUpdate;
use crate::dif::value::DIFValueContainer;
use crate::references::observers::ObserverError;
use crate::references::reference::{
    AccessError, AssignmentError, ReferenceFromValueContainerError,
    ReferenceMutability, TypeError,
};
use crate::runtime::execution::ExecutionError;
use crate::values::pointer::PointerAddress;
use datex_core::dif::reference::DIFReference;
use datex_core::dif::value::DIFReferenceNotFoundError;
use std::fmt::Display;

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

impl From<DIFReferenceNotFoundError> for DIFUpdateError {
    fn from(_: DIFReferenceNotFoundError) -> Self {
        DIFUpdateError::ReferenceNotFound
    }
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

impl From<DIFReferenceNotFoundError> for DIFCreatePointerError {
    fn from(_: DIFReferenceNotFoundError) -> Self {
        DIFCreatePointerError::ReferenceNotFound
    }
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

#[derive(Debug)]
pub enum DIFFreeError {
    ReferenceNotFound,
}
impl Display for DIFFreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DIFFreeError::ReferenceNotFound => {
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
        &self,
        address: PointerAddress,
        update: DIFUpdate,
    ) -> Result<(), DIFUpdateError>;

    /// Executes an apply operation, applying the `value` to the `callee`.
    fn apply(
        &self,
        callee: DIFValueContainer,
        value: DIFValueContainer,
    ) -> Result<DIFValueContainer, DIFApplyError>;

    /// Creates a new pointer and stores it in memory.
    /// Returns the address of the newly created pointer.
    fn create_pointer(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeContainer>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError>;

    /// Resolves a pointer address of a pointer that may not be in memory.
    /// If the pointer is not in memory, it will be loaded from external storage.
    async fn resolve_pointer_address_external(
        &self,
        address: PointerAddress,
    ) -> Result<DIFReference, DIFResolveReferenceError>;

    /// Resolves a pointer address of a pointer that is currently in memory.
    /// Returns an error if the pointer is not found in memory.
    fn resolve_pointer_address_in_memory(
        &self,
        address: PointerAddress,
    ) -> Result<DIFReference, DIFResolveReferenceError>;

    /// Starts observing changes to the pointer at the given address.
    /// As long as the pointer is observed, it will not be garbage collected.
    fn observe_pointer<F: Fn(&DIFUpdate) + 'static>(
        &self,
        address: PointerAddress,
        observer: F,
    ) -> Result<u32, DIFObserveError>;

    /// Stops observing changes to the pointer at the given address.
    /// If no other references to the pointer exist, it may be garbage collected after this call.
    fn unobserve_pointer(
        &self,
        address: PointerAddress,
        observer_id: u32,
    ) -> Result<(), DIFObserveError>;
}
