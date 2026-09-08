use crate::{
    preludes::derive::DatexNative,
    values::core_values::{
        decimal::typed_decimal::TypedDecimal, native::DatexNativeOps,
    },
};
use core::any::Any;

impl DatexNative for TypedDecimal {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl DatexNativeOps for TypedDecimal {}
