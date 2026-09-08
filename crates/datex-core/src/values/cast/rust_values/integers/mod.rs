mod try_from_core_value;
pub mod try_from_core_value_sized;

use crate::traits::{datex_hash::impl_datex_hash, value_access::ValueAccess};
mod to_instructions;

#[cfg(feature = "ast")]
mod to_datex_expression_data {
    use crate::{
        ast::expressions::DatexExpressionData,
        traits::to_datex_expression_data::ToDatexExpressionData,
        values::core_values::integer::typed_integer::TypedInteger,
    };

    impl ToDatexExpressionData for u8 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::U8(*self))
        }
    }

    impl ToDatexExpressionData for u16 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::U16(*self))
        }
    }

    impl ToDatexExpressionData for u32 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::U32(*self))
        }
    }

    impl ToDatexExpressionData for u64 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::U64(*self))
        }
    }

    impl ToDatexExpressionData for u128 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::U128(*self))
        }
    }

    impl ToDatexExpressionData for i8 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::I8(*self))
        }
    }

    impl ToDatexExpressionData for i16 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::I16(*self))
        }
    }

    impl ToDatexExpressionData for i32 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::I32(*self))
        }
    }

    impl ToDatexExpressionData for i64 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::I64(*self))
        }
    }

    impl ToDatexExpressionData for i128 {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            DatexExpressionData::TypedInteger(TypedInteger::I128(*self))
        }
    }

    impl ToDatexExpressionData for usize {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            #[cfg(target_pointer_width = "32")]
            {
                DatexExpressionData::TypedInteger(TypedInteger::U32(
                    *self as u32,
                ))
            }
            #[cfg(target_pointer_width = "64")]
            {
                DatexExpressionData::TypedInteger(TypedInteger::U64(
                    *self as u64,
                ))
            }
        }
    }

    impl ToDatexExpressionData for isize {
        fn to_datex_expression_data(&self) -> DatexExpressionData {
            #[cfg(target_pointer_width = "32")]
            {
                DatexExpressionData::TypedInteger(TypedInteger::I32(
                    *self as i32,
                ))
            }
            #[cfg(target_pointer_width = "64")]
            {
                DatexExpressionData::TypedInteger(TypedInteger::I64(
                    *self as i64,
                ))
            }
        }
    }
}

impl ValueAccess for u8 {}
impl ValueAccess for u16 {}
impl ValueAccess for u32 {}
impl ValueAccess for u64 {}
impl ValueAccess for u128 {}
impl ValueAccess for i8 {}
impl ValueAccess for i16 {}
impl ValueAccess for i32 {}
impl ValueAccess for i64 {}
impl ValueAccess for i128 {}
impl ValueAccess for usize {}
impl ValueAccess for isize {}

impl_datex_hash!(u8);
impl_datex_hash!(u16);
impl_datex_hash!(u32);
impl_datex_hash!(u64);
impl_datex_hash!(u128);
impl_datex_hash!(i8);
impl_datex_hash!(i16);
impl_datex_hash!(i32);
impl_datex_hash!(i64);
impl_datex_hash!(i128);
impl_datex_hash!(usize);
impl_datex_hash!(isize);
