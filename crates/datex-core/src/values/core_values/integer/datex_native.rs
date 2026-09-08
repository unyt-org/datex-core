use crate::{
    prelude::*,
    values::core_values::{
        integer::Integer,
        native::{DatexNative, DatexNativeOps, add_native_impl},
    },
};
use core::any::Any;

impl DatexNative for Integer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl DatexNativeOps for Integer {
    fn add_native(
        &self,
        rhs: &dyn DatexNative,
    ) -> Option<Box<dyn DatexNative>> {
        add_native_impl(self, rhs)
    }
}
