pub mod convert_value;

use crate::{
    dif::deserialize_serde_context::impl_serde_with_context,
    traits::{datex_hash::impl_datex_hash, value_access::ValueAccess},
};
mod to_instructions;
mod try_clone;

impl ValueAccess for bool {}

#[cfg(feature = "ast")]
mod to_datex_expression_data {
    use crate::{
        ast::expressions::DatexExpressionData,
        traits::to_datex_expression_data::ToDatexExpressionData,
        values::core_values::boolean::Boolean,
    };

    impl ToDatexExpressionData for bool {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::Boolean(Boolean(*self))
        }
    }
}
impl_datex_hash!(bool);
impl_serde_with_context!(bool);
