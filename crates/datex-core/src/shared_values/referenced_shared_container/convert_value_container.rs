use crate::{
    shared_values::{ReferencedSharedContainer, SharedContainer},
    traits::convert_value_container::ConvertValueContainer,
    values::value_container::{
        ValueContainer,
        borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};

impl ConvertValueContainer for ReferencedSharedContainer {
    fn to_value_container(self) -> ValueContainer {
        ValueContainer::Shared(SharedContainer::Referenced(self))
    }

    fn as_borrowed_value_container(&self) -> BorrowedValueContainer<'_> {
        BorrowedValueContainer::Shared(SharedContainer::Referenced(
            self.clone(),
        ))
    }
    fn as_borrowed_value_container_mut(
        &mut self,
    ) -> BorrowedValueContainerMut<'_> {
        BorrowedValueContainerMut::Shared(SharedContainer::Referenced(
            self.clone(),
        ))
    }

    fn try_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ValueContainer>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Shared(shared_container) => {
                Ok(shared_container.derive_reference_with_max_mutability())
            }
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
            ValueContainer::Shared(SharedContainer::Referenced(
                referenced_shared_container,
            )) => Ok(referenced_shared_container),
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
            ValueContainer::Shared(SharedContainer::Referenced(
                referenced_shared_container,
            )) => Ok(referenced_shared_container),
            _ => Err(()),
        }
    }
}
