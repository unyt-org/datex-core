use crate::{
    traits::convert_value_container::ConvertValueContainer,
    values::{
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
        value_container::{
            ValueContainer,
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
        },
    },
};

impl ConvertValueContainer for Value {
    fn to_value_container(self) -> ValueContainer {
        ValueContainer::Local(self)
    }

    fn as_borrowed_value_container(&self) -> BorrowedValueContainer<'_> {
        BorrowedValueContainer::Local(BorrowedValue::from(self))
    }
    fn as_borrowed_value_container_mut(
        &mut self,
    ) -> BorrowedValueContainerMut<'_> {
        BorrowedValueContainerMut::Local(BorrowedValueMut::from(self))
    }

    fn try_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ValueContainer>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Local(value) => Ok(value),
            _ => Err(value_container),
        }
    }

    fn try_borrow_from_value_container(
        value_container: &ValueContainer,
    ) -> Result<&Self, ()>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Local(value) => Ok(value),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value_container(
        value_container: &mut ValueContainer,
    ) -> Result<&mut Self, ()>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Local(value) => Ok(value),
            _ => Err(()),
        }
    }
}
