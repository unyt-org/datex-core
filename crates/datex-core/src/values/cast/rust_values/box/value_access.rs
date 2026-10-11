use crate::{
    prelude::*,
    shared_values::errors::AccessError,
    traits::value_access::ValueAccess,
    values::value_container::{
        borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
        value_key::BorrowedValueKey,
    },
};
use core::ops::{Deref, DerefMut};

impl<T: ValueAccess> ValueAccess for Box<T> {
    fn try_get_property(
        &self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        self.deref().try_get_property(key)
    }
    fn try_get_property_mut(
        &mut self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        self.deref_mut().try_get_property_mut(key)
    }
}
