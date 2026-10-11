use crate::{
    prelude::*,
    traits::iter_parts::IterParts,
    values::{
        core_value::CoreValue,
        value_container::borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};

impl IterParts for CoreValue {
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        match self {
            CoreValue::List(list) => list.iter_list_parts(),
            CoreValue::Range(range) => range.iter_list_parts(),
            _ => None,
        }
    }

    fn iter_list_parts_mut<'a>(
        &'a mut self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
    {
        match self {
            CoreValue::List(list) => list.iter_list_parts_mut(),
            CoreValue::Range(range) => range.iter_list_parts_mut(),
            _ => None,
        }
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
        match self {
            CoreValue::Map(map) => map.iter_map_parts(),
            _ => None,
        }
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
        match self {
            CoreValue::Map(map) => map.iter_map_parts_mut(),
            _ => None,
        }
    }
}
