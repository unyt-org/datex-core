use crate::{
    preludes::derive::DatexNative,
    values::core_values::{decimal::Decimal, native::DatexNativeOps},
};
use core::any::Any;

impl DatexNative for Decimal {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl DatexNativeOps for Decimal {}
