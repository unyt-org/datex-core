pub mod classification;
mod convert_value;
pub mod datex_hash;
pub mod datex_native;

use crate::{
    prelude::*,
    traits::{
        convert_parts::{FromParts, HasPartsKind, IntoParts},
        convert_value_container::ConvertValueContainer,
        datex_native_only_structural::DatexNativeOnlyStructural,
        datex_native_structural::DatexNativeStructural,
        get_core_lib_type_id::GetCoreLibTypeId,
        get_datex_type::GetDatexType,
        value_access::ValueAccess,
    },
    value_updates::update_handler::{
        UpdateCallbackDataAccess, UpdateHandlerImpl,
    },
    values::value_container::ValueContainer,
};
use core::time::Duration;

mod to_instructions;
#[cfg(feature = "ast")]
mod to_datex_expression_data {
    use crate::{
        ast::expressions::DatexExpressionData,
        traits::to_datex_expression_data::ToDatexExpressionData,
        values::core_values::integer::Integer,
    };
    use core::time::Duration;

    impl ToDatexExpressionData for Duration {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            // TODO: use amount once implemented
            DatexExpressionData::Integer(Integer::from(self.as_millis()))
        }
    }
}

impl ValueAccess for Duration {}
impl FromParts for Duration {
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
impl IntoParts for Duration {
    fn try_into_single_value<'a>(self: Box<Self>) -> Result<ValueContainer, ()>
    where
        Self: 'a,
    {
        Ok(self.to_value_container())
    }
}
impl HasPartsKind for Duration {}
impl GetCoreLibTypeId for Duration {}
impl GetDatexType for Duration {}
impl UpdateHandlerImpl for Duration {}
impl UpdateCallbackDataAccess for Duration {}

impl DatexNativeStructural for Duration {}
impl DatexNativeOnlyStructural for Duration {}
