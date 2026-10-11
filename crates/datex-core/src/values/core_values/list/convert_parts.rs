use crate::{
    prelude::*,
    traits::convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
    values::core_values::list::List,
};

impl HasPartsKind for List {
    fn parts_kind(&self) -> PartsKind {
        PartsKind::List
    }
}

impl IntoParts for List {
    fn try_into_list_parts<'a>(self: Box<Self>) -> Result<List, ()>
    where
        Self: 'a,
    {
        Ok(*self)
    }
}

impl FromParts for List {
    fn try_from_list_parts_with_tag(
        parts: List,
        _tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(parts)
    }
}
