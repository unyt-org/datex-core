use crate::{
    prelude::*,
    traits::convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        core_values::{list::List, map::Map},
        value::value_classification::ValueClassification,
        value_container::ValueContainer,
    },
};

impl HasPartsKind for CoreValueWithClassification {
    fn parts_kind(&self) -> PartsKind {
        self.inner.parts_kind()
    }
}

impl FromParts for CoreValueWithClassification {
    fn try_from_map_parts_with_tag(
        parts: Map,
        tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(CoreValueWithClassification {
            inner: CoreValue::try_from_map_parts_with_tag(parts, None)?,
            classification: ValueClassification::new_with_maybe_tag(tag),
        })
    }

    fn try_from_list_parts_with_tag(
        parts: List,
        tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(CoreValueWithClassification {
            inner: CoreValue::try_from_list_parts_with_tag(parts, None)?,
            classification: ValueClassification::new_with_maybe_tag(tag),
        })
    }

    fn try_from_single_value_with_tag(
        value: ValueContainer,
        tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Ok(CoreValueWithClassification {
            inner: CoreValue::try_from_single_value_with_tag(value, None)?,
            classification: ValueClassification::new_with_maybe_tag(tag),
        })
    }
}
impl IntoParts for CoreValueWithClassification {
    fn try_into_map_parts<'a>(self: Box<Self>) -> Result<Map, ()>
    where
        Self: 'a,
    {
        Box::new(self.inner).try_into_map_parts()
    }

    fn try_into_list_parts<'a>(self: Box<Self>) -> Result<List, ()>
    where
        Self: 'a,
    {
        Box::new(self.inner).try_into_list_parts()
    }

    fn try_into_single_value<'a>(self: Box<Self>) -> Result<ValueContainer, ()>
    where
        Self: 'a,
    {
        Box::new(self.inner).try_into_single_value()
    }
}
