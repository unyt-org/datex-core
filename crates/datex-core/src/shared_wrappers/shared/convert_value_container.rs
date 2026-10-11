use crate::{
    shared_wrappers::shared::Shared,
    traits::convert_value_container::ConvertValueContainer,
    values::{
        core_values::native::DatexNative,
        value_container::{
            ValueContainer,
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
        },
    },
};

impl<T> ConvertValueContainer for Shared<T>
where
    T: ConvertValueContainer + DatexNative,
{
    fn to_value_container(self) -> ValueContainer {
        ValueContainer::Shared(self.container)
    }

    fn as_borrowed_value_container<'a>(&'a self) -> BorrowedValueContainer<'a> {
        BorrowedValueContainer::Shared(self.container.clone())
    }
    fn as_borrowed_value_container_mut(
        &mut self,
    ) -> BorrowedValueContainerMut<'_> {
        BorrowedValueContainerMut::Shared(self.container.clone())
    }

    fn try_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ValueContainer>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Shared(container) => {
                // TODO: no clone?
                Shared::try_from(container.clone())
                    .map_err(|_| ValueContainer::Shared(container))
            }
            _ => Err(value_container),
        }
    }

    fn try_borrow_from_value_container(
        _value_container: &ValueContainer,
    ) -> Result<&Self, ()>
    where
        Self: Sized,
    {
        Err(())
    }

    fn try_borrow_mut_from_value_container(
        _value_container: &mut ValueContainer,
    ) -> Result<&mut Self, ()>
    where
        Self: Sized,
    {
        Err(())
    }
}
