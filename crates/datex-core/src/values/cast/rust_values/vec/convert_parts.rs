use crate::{
    prelude::*,
    traits::{
        convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
        convert_value_container::ConvertValueContainer,
    },
    values::core_values::list::List,
};

impl<T> HasPartsKind for Vec<T> {
    fn parts_kind(&self) -> PartsKind {
        PartsKind::List
    }
}
impl<T: ConvertValueContainer> IntoParts for Vec<T> {
    fn try_into_list_parts<'a>(self: Box<Self>) -> Result<List, ()>
    where
        Self: 'a,
    {
        let mut list = List::default();
        for item in *self {
            list.push(item.to_value_container());
        }
        Ok(list)
    }
}

impl<T: ConvertValueContainer + FromParts> FromParts for Vec<T> {
    fn try_from_list_parts_with_tag(
        parts: List,
        _tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        let mut vec = Vec::new();
        for item in parts {
            vec.push(T::try_convert_from_value_container(item).map_err(|_| ())?);
        }
        Ok(vec)
    }
}
