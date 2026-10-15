use crate::{
    shared_values::errors::AccessError,
    traits::value_access::ValueAccess,
    values::{
        core_values::list::List,
        value_container::{
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
        },
    },
};
use crate::values::value_container::ValueContainer;

impl ValueAccess for List {
    fn try_get_property(
        &self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        if let Some(index) = key.try_as::<u32>() {
            Ok(self.try_get(*index as i64)?.into()) // FIXME: int conversion
        } else {
            Err(AccessError::InvalidIndexKey)
        }
    }

    fn try_get_property_mut(
        &mut self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        if let Some(index) = key.try_as::<u32>(){
            Ok(self.try_get_mut(*index as i64)?.into()) // FIXME: int conversion
        } else {
            Err(AccessError::InvalidIndexKey)
        }
    }
}
