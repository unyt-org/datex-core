use crate::{
    prelude::*,
    traits::iter_parts::IterParts,
    values::{
        core_value_with_classification::CoreValueWithClassification,
        value_container::borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};

impl IterParts for CoreValueWithClassification {
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        self.inner.iter_list_parts()
    }
    fn iter_list_parts_mut<'a>(
        &'a mut self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
    {
        self.inner.iter_list_parts_mut()
    }
    fn iter_map_parts<'a>(
        &'a self,
    ) -> Option<
        Box<
            dyn Iterator<
                    Item = (
                        BorrowedValueContainer<'a>,
                        BorrowedValueContainer<'a>,
                    ),
                > + 'a,
        >,
    > {
        self.inner.iter_map_parts()
    }
    fn iter_map_parts_mut<'a>(
        &'a mut self,
    ) -> Option<
        Box<
            dyn Iterator<
                    Item = (
                        BorrowedValueContainer<'a>,
                        BorrowedValueContainerMut<'a>,
                    ),
                > + 'a,
        >,
    > {
        self.inner.iter_map_parts_mut()
    }
}
