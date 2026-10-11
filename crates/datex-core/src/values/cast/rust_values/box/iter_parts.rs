use crate::{
    prelude::*,
    traits::iter_parts::IterParts,
    values::value_container::borrowed_value_container::{
        BorrowedValueContainer, BorrowedValueContainerMut,
    },
};

impl<T> IterParts for Box<T>
where
    T: IterParts,
{
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        self.as_ref().iter_list_parts()
    }
    fn iter_list_parts_mut<'a>(
        &'a mut self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
    {
        self.as_mut().iter_list_parts_mut()
    }
}
