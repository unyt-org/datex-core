use crate::{
    prelude::*,
    shared_values::errors::{AccessError, IndexOutOfBoundsError},
    traits::{get_datex_type::GetDatexType, value_access::ValueAccess},
    values::{
        core_values::native::DatexNativeBase,
        value_container::{
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
            value_key::BorrowedValueKey,
        },
    },
};

impl<T> ValueAccess for Vec<T>
where
    T: DatexNativeBase + GetDatexType,
{
    fn try_get_property(
        &self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        match key {
            BorrowedValueKey::Index(index) => {
                if let Some(value) = self.get(index as usize) {
                    Ok(value.as_borrowed_value_container())
                } else {
                    Err(AccessError::IndexOutOfBounds(IndexOutOfBoundsError {
                        index: index as u32,
                    }))
                }
            }
            _ => Err(AccessError::InvalidOperation(format!(
                "Invalid key: {:?}",
                key
            ))), // TODO: better access errors
        }
    }

    fn try_get_property_mut(
        &mut self,
        _key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        todo!()
    }
}
