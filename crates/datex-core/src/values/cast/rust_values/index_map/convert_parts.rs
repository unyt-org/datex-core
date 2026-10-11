use crate::{
    prelude::*,
    random::RandomState,
    traits::{
        convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
        convert_value_container::ConvertValueContainer,
    },
    values::core_values::map::Map,
};
use core::hash::Hash;
use indexmap::IndexMap;

impl<K: ConvertValueContainer, V: ConvertValueContainer> HasPartsKind
    for IndexMap<K, V, RandomState>
{
    fn parts_kind(&self) -> PartsKind {
        PartsKind::Map
    }
}

impl<K: ConvertValueContainer, V: ConvertValueContainer> IntoParts
    for IndexMap<K, V, RandomState>
{
    fn try_into_map_parts<'a>(self: Box<Self>) -> Result<Map, ()>
    where
        Self: 'a,
    {
        Ok(self
            .into_iter()
            .map(|(key, value)| {
                (key.to_value_container(), value.to_value_container())
            })
            .collect::<Map>())
    }
}

impl<K: ConvertValueContainer + Eq + Hash, V: ConvertValueContainer> FromParts
    for IndexMap<K, V, RandomState>
{
    fn try_from_map_parts_with_tag(
        parts: Map,
        _tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        let mut index_map = IndexMap::default();
        for (key, value) in parts {
            let key = K::try_from_value_container(key).map_err(|_| ())?;
            let value = V::try_from_value_container(value).map_err(|_| ())?;
            index_map.insert(key, value);
        }
        Ok(index_map)
    }
}
