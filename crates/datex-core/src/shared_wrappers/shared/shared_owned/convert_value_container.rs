use crate::{
    shared_wrappers::shared::shared_owned::Shared,
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
use crate::shared_values::SharedContainer;

impl<T> ConvertValueContainer for Shared<T>
where
    T: ConvertValueContainer + DatexNative,
{
    fn to_value_container(self) -> ValueContainer {
        ValueContainer::Shared(SharedContainer::Owned(self.into_container()))
    }

    fn as_borrowed_value_container(&self) -> BorrowedValueContainer {
        BorrowedValueContainer::Shared(SharedContainer::Referenced(self.container().derive_with_max_mutability()))
    }
    fn as_borrowed_value_container_mut(
        &mut self,
    ) -> BorrowedValueContainerMut {
        BorrowedValueContainerMut::Shared(SharedContainer::Referenced(self.container().derive_with_max_mutability()))
    }

    fn try_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ValueContainer>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Shared(SharedContainer::Owned(container)) => {
                Shared::try_from(container)
                    .map_err(|container| ValueContainer::Shared(SharedContainer::Owned(container)))
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
