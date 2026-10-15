use crate::{
    shared_values::errors::AccessError,
    traits::value_access::ValueAccess,
    values::{
        core_values::map::Map,
        value_container::{
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
        },
    },
};
use crate::values::value_container::ValueContainer;

impl ValueAccess for Map {
    fn try_get_property(
        &self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        Ok(self.try_get(key)?.into())
    }

    fn try_get_property_mut(
        &mut self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        Ok(self.try_get_mut(key)?.into())
    }
}
