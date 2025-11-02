use core::prelude::rust_2024::*;
use core::result::Result;
use crate::dif::r#type::DIFTypeContainer;
use crate::dif::update::DIFUpdateData;
use crate::dif::value::DIFValueContainer;
use crate::references::observers::{
    ObserveOptions, ObserverError, TransceiverId,
};
use crate::references::reference::{
    AccessError, AssignmentError, ReferenceCreationError, ReferenceMutability,
    TypeError,
};
use crate::runtime::execution::ExecutionError;
use crate::values::pointer::PointerAddress;
use datex_core::dif::reference::DIFReference;
use datex_core::dif::update::DIFUpdate;
use datex_core::dif::value::DIFReferenceNotFoundError;
use core::fmt::Display;

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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DIFObserveError::ReferenceNotFound => {
                core::write!(f, "Reference not found")
            }
            DIFObserveError::ObserveError(e) => {
                core::write!(f, "Observe error: {}", e)
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
    TypeError(Box<TypeError>),
}

impl From<DIFReferenceNotFoundError> for DIFUpdateError {
    fn from(_: DIFReferenceNotFoundError) -> Self {
        DIFUpdateError::ReferenceNotFound
    }
}
impl From<AccessError> for DIFUpdateError {
    fn from(err: AccessError) -> Self {
        DIFUpdateError::AccessError(err)
    }
}
impl From<AssignmentError> for DIFUpdateError {
    fn from(err: AssignmentError) -> Self {
        DIFUpdateError::AssignmentError(err)
    }
}
impl From<TypeError> for DIFUpdateError {
    fn from(err: TypeError) -> Self {
        DIFUpdateError::TypeError(Box::new(err))
    }
}

impl Display for DIFUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DIFUpdateError::ReferenceNotFound => {
                core::write!(f, "Reference not found")
            }
            DIFUpdateError::InvalidUpdate => {
                core::write!(f, "Invalid update operation")
            }
            DIFUpdateError::AccessError(e) => core::write!(f, "Access error: {}", e),
            DIFUpdateError::AssignmentError(e) => {
                core::write!(f, "Assignment error: {}", e)
            }
            DIFUpdateError::TypeError(e) => core::write!(f, "Type error: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum DIFApplyError {
    ExecutionError(ExecutionError),
    ReferenceNotFound,
}
impl Display for DIFApplyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DIFApplyError::ExecutionError(e) => {
                core::write!(f, "Execution error: {}", e)
            }
            DIFApplyError::ReferenceNotFound => {
                core::write!(f, "Reference not found")
            }
        }
    }
}

#[derive(Debug)]
pub enum DIFCreatePointerError {
    ReferenceNotFound,
    ReferenceCreationError(ReferenceCreationError),
}

impl From<DIFReferenceNotFoundError> for DIFCreatePointerError {
    fn from(_: DIFReferenceNotFoundError) -> Self {
        DIFCreatePointerError::ReferenceNotFound
    }
}

impl Display for DIFCreatePointerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DIFCreatePointerError::ReferenceNotFound => {
                core::write!(f, "Reference not found")
            }
            DIFCreatePointerError::ReferenceCreationError(e) => {
                core::write!(f, "Reference from value container error: {}", e)
            }
        }
    }
}

#[derive(Debug)]
pub enum DIFResolveReferenceError {
    ReferenceNotFound,
}
impl Display for DIFResolveReferenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DIFResolveReferenceError::ReferenceNotFound => {
                core::write!(f, "Reference not found")
            }
        }
    }
}

impl From<ReferenceCreationError> for DIFCreatePointerError {
    fn from(err: ReferenceCreationError) -> Self {
        DIFCreatePointerError::ReferenceCreationError(err)
    }
}

pub trait DIFInterface {
    /// Applies a DIF update to the value at the given pointer address.
    fn update(
        &self,
        source_id: TransceiverId,
        address: PointerAddress,
        update: DIFUpdateData,
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
    fn resolve_pointer_address_external(
        &self,
        address: PointerAddress,
    ) -> impl Future<Output = Result<DIFReference, DIFResolveReferenceError>>;

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
        transceiver_id: TransceiverId,
        address: PointerAddress,
        options: ObserveOptions,
        observer: F,
    ) -> Result<u32, DIFObserveError>;

    /// Updates the options for an existing observer on the pointer at the given address.
    /// If the observer does not exist, an error is returned.
    fn update_observer_options(
        &self,
        address: PointerAddress,
        observer_id: u32,
        options: ObserveOptions,
    ) -> Result<(), DIFObserveError>;

    /// Stops observing changes to the pointer at the given address.
    /// If no other references to the pointer exist, it may be garbage collected after this call.
    fn unobserve_pointer(
        &self,
        address: PointerAddress,
        observer_id: u32,
    ) -> Result<(), DIFObserveError>;
}
