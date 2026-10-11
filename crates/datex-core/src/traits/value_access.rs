use crate::{
    prelude::*,
    shared_values::errors::AccessError,
    values::value_container::{
        borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
        value_key::BorrowedValueKey,
    },
};

/// Trait for accessing properties of a value as [ValueContainers] or [BorrowedValueContainer]. This is used for accessing properties of values in a generic way, such as for maps and structs.
pub trait ValueAccess {
    /// Gets a reference to a property on the value if applicable (e.g. for map and structs).
    /// This method returns a [BorrowedValueContainer] which can be either a [ValueContainer] or a [Callable] value.
    fn try_get_property(
        &self,
        _key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        Err(AccessError::InvalidOperation(
            "Cannot get property".to_string(),
        ))
    }

    /// Gets a mutable reference to a property on the value if applicable (e.g. for map and structs)
    fn try_get_property_mut(
        &mut self,
        _key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        Err(AccessError::InvalidOperation(
            "Cannot get property".to_string(),
        ))
    }
}
