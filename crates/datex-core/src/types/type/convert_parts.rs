use crate::{
    prelude::*,
    traits::{
        convert_parts::{FromParts, HasPartsKind, IntoParts},
        convert_value_container::ConvertValueContainer,
    },
    types::r#type::Type,
    values::value_container::ValueContainer,
};

/// Default implementations - cannot be split into parts
impl IntoParts for Type {
    fn try_into_single_value<'a>(self: Box<Self>) -> Result<ValueContainer, ()>
    where
        Self: 'a,
    {
        Ok(self.to_value_container())
    }
}
impl FromParts for Type {
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
impl HasPartsKind for Type {}
