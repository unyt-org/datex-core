use crate::{
    prelude::*,
    traits::{
        convert_parts::{FromParts, HasPartsKind, IntoParts},
        convert_value_container::ConvertValueContainer,
    },
    values::{core_values::range::Range, value_container::ValueContainer},
};

impl FromParts for Range {
    fn try_from_single_value_with_tag(
        value: ValueContainer,
        _tag: Option<&str>,
    ) -> Result<Self, ()>
    where
        Self: Sized,
    {
        Self::try_from_value_container(value).map_err(|_| ())
    }
}
impl IntoParts for Range {
    fn try_into_single_value<'a>(self: Box<Self>) -> Result<ValueContainer, ()>
    where
        Self: 'a,
    {
        Ok(self.to_value_container())
    }
}
impl HasPartsKind for Range {}
