use crate::{
    preludes::derive::DatexNative,
    values::core_values::{
        integer::typed_integer::TypedInteger, native::DatexNativeOps,
    },
};
use core::any::Any;

impl DatexNative for TypedInteger {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl DatexNativeOps for TypedInteger {}
