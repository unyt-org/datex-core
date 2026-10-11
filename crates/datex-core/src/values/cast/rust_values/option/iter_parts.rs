use crate::{
    prelude::*,
    traits::iter_parts::IterParts,
    values::value_container::borrowed_value_container::{
        BorrowedValueContainer, BorrowedValueContainerMut,
    },
};

impl<T: IterParts> IterParts for Option<T> {
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        match self {
            Some(value) => value.iter_list_parts(),
            None => None,
        }
    }
    fn iter_list_parts_mut<'a>(
        &'a mut self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
    {
        match self {
            Some(value) => value.iter_list_parts_mut(),
            None => None,
        }
    }
}
