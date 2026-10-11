use crate::{
    prelude::*,
    traits::convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
    values::{
        core_values::{list::List, map::Map},
        value::Value,
        value_container::ValueContainer,
    },
};

impl HasPartsKind for ValueContainer {
    fn parts_kind(&self) -> PartsKind {
        match self {
            ValueContainer::Local(value) => value.parts_kind(),
            ValueContainer::Shared(_shared) => PartsKind::None,
        }
    }
}

impl IntoParts for ValueContainer {
    fn try_into_map_parts<'a>(self: Box<Self>) -> Result<Map, ()>
    where
        Self: 'a,
    {
        match self {
            ValueContainer::Local(value) => {
                Box::new(value).try_into_map_parts()
            }
            ValueContainer::Shared(_shared) => Err(()),
        }
    }

    fn try_into_list_parts<'a>(self: Box<Self>) -> Result<List, ()>
    where
        Self: 'a,
    {
        match self {
            ValueContainer::Local(value) => {
                Box::new(value).try_into_list_parts()
            }
            ValueContainer::Shared(_shared) => Err(()),
        }
    }

    fn try_into_single_value<'a>(self: Box<Self>) -> Result<ValueContainer, ()>
    where
        Self: 'a,
    {
        Ok(*self)
    }
}

impl FromParts for ValueContainer {
    fn try_from_map_parts_with_tag(
        parts: Map,
        tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(ValueContainer::Local(Value::try_from_map_parts_with_tag(
            parts, tag,
        )?))
    }
    fn try_from_list_parts_with_tag(
        parts: List,
        tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(ValueContainer::Local(Value::try_from_list_parts_with_tag(
            parts, tag,
        )?))
    }

    fn try_from_single_value_with_tag(
        value: ValueContainer,
        _tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(value)
    }
}
