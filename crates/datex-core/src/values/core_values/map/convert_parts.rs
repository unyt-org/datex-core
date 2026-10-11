use crate::{
    prelude::*,
    traits::convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
    values::core_values::map::Map,
};

impl HasPartsKind for Map {
    fn parts_kind(&self) -> PartsKind {
        PartsKind::Map
    }
}

impl IntoParts for Map {
    fn try_into_map_parts<'a>(self: Box<Self>) -> Result<Map, ()>
    where
        Self: 'a,
    {
        Ok(*self)
    }
}

impl FromParts for Map {
    fn try_from_map_parts_with_tag(
        parts: Map,
        _tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(parts)
    }
}
