pub mod to_instructions;

use crate::{
    shared_values::SharedContainer, types::r#type::Type,
    values::core_value::CoreValue,
};
use core::ops::Deref;

/// A wrapper around an [SharedContainer] which guarantees
/// that the contained value is always a [CoreValue::Type]
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct SharedContainerContainingType(SharedContainer);

impl Deref for SharedContainerContainingType {
    type Target = SharedContainer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SharedContainerContainingType> for SharedContainer {
    fn from(value: SharedContainerContainingType) -> Self {
        value.0
    }
}

impl SharedContainerContainingType {
    /// Creates a new [SharedContainerContainingType] from a [SharedContainer] without checking the constraint.
    /// # Safety
    /// The caller must ensure that the constraint for [SharedContainerContainingType] is satisfied
    /// (i.e. the allowed type of the container is a [TypeDefinition::Type])
    pub unsafe fn new_unchecked(container: SharedContainer) -> Self {
        SharedContainerContainingType(container)
    }

    /// Calls the provided callback with a reference to the recursively collapsed inner [Type] value of the shared container
    /// The [SharedContainerContainingType] guarantees that the inner value is always a [CoreValue::Type], so this method can never panic.
    pub fn with_collapsed_type_value<R>(
        &self,
        f: impl FnOnce(&Type) -> R,
    ) -> R {
        let val = self.0.collapsed_value();
        let val_sheep = val.borrow();
        let ty = match val_sheep.try_as_core_value() {
            Some(CoreValue::Type(ty)) => ty,
            _ => unreachable!(
                "The constraint for SharedContainerContainingType guarantees that the inner value is always a CoreValue::Type"
            ),
        };
        f(ty)
    }
}

impl TryFrom<SharedContainer> for SharedContainerContainingType {
    type Error = ();

    fn try_from(value: SharedContainer) -> Result<Self, Self::Error> {
        let is_type = {
            let val = value.collapsed_value();
            matches!(
                &val.borrow().try_as_core_value(),
                Some(CoreValue::Type(_))
            )
        };
        if is_type {
            Ok(SharedContainerContainingType(value))
        } else {
            Err(())
        }
    }
}
