mod convert_value;
mod datex_hash;

use crate::traits::value_access::ValueAccess;
pub mod serde_dif;
mod to_instructions;
mod try_clone;

#[cfg(feature = "ast")]
mod to_datex_expression_data {
    use crate::{
        ast::expressions::DatexExpressionData,
        traits::to_datex_expression_data::ToDatexExpressionData,
        values::core_values::decimal::typed_decimal::TypedDecimal,
    };
    use ordered_float::OrderedFloat;

    impl ToDatexExpressionData for f32 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedDecimal(TypedDecimal::F32(OrderedFloat(
                *self,
            )))
        }
    }

    impl ToDatexExpressionData for f64 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedDecimal(TypedDecimal::F64(OrderedFloat(
                *self,
            )))
        }
    }
}

impl ValueAccess for f32 {}
impl ValueAccess for f64 {}
