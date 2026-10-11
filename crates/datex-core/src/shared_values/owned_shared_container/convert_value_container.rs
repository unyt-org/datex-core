use crate::{
    shared_values::{OwnedSharedContainer, SharedContainer},
    traits::convert_value_container::ConvertValueContainer,
    values::value_container::{
        ValueContainer,
        borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};

impl ConvertValueContainer for OwnedSharedContainer {
    fn to_value_container(self) -> ValueContainer {
        ValueContainer::Shared(SharedContainer::Owned(self))
    }

    fn as_borrowed_value_container(&self) -> BorrowedValueContainer<'_> {
        BorrowedValueContainer::Shared(SharedContainer::Referenced(
            self.derive_with_max_mutability(),
        ))
    }
    fn as_borrowed_value_container_mut(
        &mut self,
    ) -> BorrowedValueContainerMut<'_> {
        BorrowedValueContainerMut::Shared(SharedContainer::Referenced(
            self.derive_with_max_mutability(),
        ))
    }

    fn try_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ValueContainer>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Shared(SharedContainer::Owned(
                owned_shared_container,
            )) => Ok(owned_shared_container),
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
            ValueContainer::Shared(SharedContainer::Owned(
                owned_shared_container,
            )) => Ok(owned_shared_container),
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
            ValueContainer::Shared(SharedContainer::Owned(
                owned_shared_container,
            )) => Ok(owned_shared_container),
            _ => Err(()),
        }
    }
}
