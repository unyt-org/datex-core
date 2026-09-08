use crate::{
    prelude::*,
    preludes::derive::{DatexNative, StaticClassification},
    traits::get_datex_type::GetDatexType,
    values::core_values::native::DatexNativeOps,
};
use core::any::Any;

impl<T: DatexNative + GetDatexType + StaticClassification> DatexNative
    for Box<T>
{
    fn as_any(&self) -> &dyn Any {
        self.as_ref().as_any() // FIXME self or T
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.as_mut().as_any_mut() // FIXME self or T 
    }
}
impl<T: DatexNative + GetDatexType + StaticClassification> DatexNativeOps
    for Box<T>
{
}
