use crate::{
    shared_values::errors::AccessError,
    traits::value_access::ValueAccess,
    values::{
        core_values::list::List,
        value_container::{
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
            value_key::BorrowedValueKey,
        },
    },
};

impl ValueAccess for List {
    fn try_get_property(
        &self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        if let Some(index) = key.try_as_index() {
            Ok(self.try_get(index)?.into())
        } else {
            Err(AccessError::InvalidIndexKey)
        }
    }

    fn try_get_property_mut(
        &mut self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        if let Some(index) = key.try_as_index() {
            Ok(self.try_get_mut(index)?.into())
        } else {
            Err(AccessError::InvalidIndexKey)
        }
    }
}
