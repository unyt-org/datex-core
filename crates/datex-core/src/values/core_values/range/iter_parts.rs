use crate::{
    prelude::*,
    traits::iter_parts::IterParts,
    values::{
        core_values::range::Range,
        value_container::borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};

impl IterParts for Range {
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        Some(Box::new(gen {
            yield self.start.as_ref().into();
            yield self.end.as_ref().into();
        }))
    }

    fn iter_list_parts_mut<'a>(
        &'a mut self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
    {
        Some(Box::new(gen {
            yield self.start.as_mut().into();
            yield self.end.as_mut().into();
        }))
    }
}
