use crate::{
    shared_values::errors::AccessError,
    traits::value_access::ValueAccess,
    values::{
        core_values::map::Map,
        value_container::{
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
            value_key::BorrowedValueKey,
        },
    },
};

impl ValueAccess for Map {
    fn try_get_property(
        &self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        Ok(self.try_get(key)?.into())
    }

    fn try_get_property_mut(
        &mut self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        Ok(self.try_get_mut(key)?.into())
    }
}
