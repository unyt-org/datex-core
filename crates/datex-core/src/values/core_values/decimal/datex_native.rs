use crate::{
    preludes::derive::DatexNative,
    values::core_values::{
        decimal::Decimal,
        native::{DatexNativeOps, add_native_impl},
    },
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

impl DatexNativeOps for Decimal {
    fn add_native(
        &self,
        rhs: &dyn DatexNative,
    ) -> Option<Box<dyn DatexNative>> {
        add_native_impl(self, rhs)
    }
}
