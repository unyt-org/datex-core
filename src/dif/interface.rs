use crate::dif::DIFUpdate;
use crate::dif::value::DIFValueContainer;
use crate::references::observers::{ObserverError, ReferenceObserver};
use crate::references::reference::{
    AccessError, AssignmentError, ReferenceFromValueContainerError, TypeError,
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

#[derive(Debug)]
pub enum DIFUpdateError {
    ReferenceNotFound,
    InvalidUpdate,
    AccessError(AccessError),
    AssignmentError(AssignmentError),
    TypeError(TypeError),
}

#[derive(Debug)]
pub enum DIFApplyError {
    ExecutionError(ExecutionError),
    ReferenceNotFound,
}

#[derive(Debug)]
pub enum DIFCreatePointerError {
    ReferenceNotFound,
    ReferenceFromValueContainerError(ReferenceFromValueContainerError),
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
    fn create_pointer(
        &self,
        value: DIFValueContainer,
    ) -> Result<PointerAddress, DIFCreatePointerError>;

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
