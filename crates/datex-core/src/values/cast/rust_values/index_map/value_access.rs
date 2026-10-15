use crate::{
    random::RandomState,
    shared_values::errors::AccessError,
    traits::{
        convert_value_container::ConvertValueContainer,
        value_access::ValueAccess,
    },
    values::value_container::{
        borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};
use core::hash::Hash;
use indexmap::IndexMap;
use crate::values::value_container::ValueContainer;

impl<K, V> ValueAccess for IndexMap<K, V, RandomState>
where
    K: ConvertValueContainer + Eq + Hash,
    V: ConvertValueContainer,
{
    fn try_get_property(
        &self,
        _key: &ValueContainer,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        todo!()
    }

    fn try_get_property_mut(
        &mut self,
        _key: &ValueContainer,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        todo!()
    }
}
