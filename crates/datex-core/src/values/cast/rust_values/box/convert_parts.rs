use crate::{
    prelude::*,
    traits::convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
    values::{
        core_values::{list::List, map::Map},
        value_container::ValueContainer,
    },
};
use core::ops::Deref;

impl<T: HasPartsKind> HasPartsKind for Box<T> {
    fn parts_kind(&self) -> PartsKind {
        self.deref().parts_kind()
    }
}

impl<T: IntoParts> IntoParts for Box<T> {
    fn try_into_map_parts<'a>(self: Box<Self>) -> Result<Map, ()>
    where
        Self: 'a,
    {
        let inner = *self;
        inner.try_into_map_parts()
    }

    fn try_into_list_parts<'a>(self: Box<Self>) -> Result<List, ()>
    where
        Self: 'a,
    {
        let inner = *self;
        inner.try_into_list_parts()
    }

    fn try_into_single_value<'a>(self: Box<Self>) -> Result<ValueContainer, ()>
    where
        Self: 'a,
    {
        let inner = *self;
        inner.try_into_single_value()
    }
}

impl<T: FromParts> FromParts for Box<T> {
    fn try_from_map_parts_with_tag(
        _parts: Map,
        _tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(Box::new(T::try_from_map_parts_with_tag(_parts, _tag)?))
    }

    fn try_from_list_parts_with_tag(
        parts: List,
        tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(Box::new(T::try_from_list_parts_with_tag(parts, tag)?))
    }

    fn try_from_single_value_with_tag(
        value: ValueContainer,
        tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(Box::new(T::try_from_single_value_with_tag(value, tag)?))
    }
}
