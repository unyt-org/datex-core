use crate::{
    prelude::*,
    traits::{
        convert_value_container::ConvertValueContainer, iter_parts::IterParts,
    },
    values::value_container::borrowed_value_container::{
        BorrowedValueContainer, BorrowedValueContainerMut,
    },
};

impl<T: ConvertValueContainer> IterParts for Vec<T> {
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        Some(Box::new(
            self.iter().map(|item| item.as_borrowed_value_container()),
        ))
    }
    fn iter_list_parts_mut<'a>(
        &'a mut self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
    {
        Some(Box::new(
            self.iter_mut()
                .map(|item| item.as_borrowed_value_container_mut()),
        ))
    }
}
