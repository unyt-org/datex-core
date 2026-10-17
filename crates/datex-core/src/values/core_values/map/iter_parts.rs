use crate::{
    prelude::*,
    traits::iter_parts::IterParts,
    values::{
        core_values::map::Map,
        value_container::borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};

impl IterParts for Map {
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
        Some(Box::new(gen move {
            for (key, value) in self.iter() {
                yield (key.into(), value.into());
            }
        }))
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
        Some(Box::new(gen move {
            for (key, value) in self.iter_mut() {
                yield (key.into(), value.into());
            }
        }))
    }
}
