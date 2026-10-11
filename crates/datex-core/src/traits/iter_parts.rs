use crate::{
    prelude::*,
    traits::convert_parts::{HasPartsKind, PartsKind},
    values::value_container::borrowed_value_container::{
        BorrowedValueContainer, BorrowedValueContainerMut,
    },
};

/// Trait for types that can provide an iterator over their child value containers
pub trait IterParts: HasPartsKind {
    fn iter_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        match self.parts_kind() {
            PartsKind::List => self.iter_list_parts(),
            PartsKind::Map => Some(Box::new(gen move {
                for (key, value) in self.iter_map_parts().unwrap() {
                    yield key;
                    yield value;
                }
            })),
            _ => None,
        }
    }
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        None
    }

    fn iter_list_parts_mut<'a>(
        &'a mut self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
    {
        None
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
        None
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
        None
    }
}
