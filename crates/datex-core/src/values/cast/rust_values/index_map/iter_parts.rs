use crate::{
    prelude::*,
    random::RandomState,
    traits::{
        convert_value_container::ConvertValueContainer, iter_parts::IterParts,
    },
    values::value_container::borrowed_value_container::BorrowedValueContainer,
};
use indexmap::IndexMap;

impl<K: ConvertValueContainer, V: ConvertValueContainer> IterParts
    for IndexMap<K, V, RandomState>
{
    fn iter_list_parts<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
        Some(Box::new(self.iter().flat_map(|(k, v)| {
            vec![
                k.as_borrowed_value_container(),
                v.as_borrowed_value_container(),
            ]
            .into_iter()
        })))
    }
}
