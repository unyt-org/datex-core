use std::any::Any;
use std::fmt::Display;

use super::datex_type::DatexType;
use super::datex_value::DatexValue;

pub trait AddAssignable: Any + Send + Sync {
    fn add_assign_boxed(&mut self, other: &dyn Value) -> Option<()>;
}

pub trait Value: Display + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn cast_to(&self, target: DatexType) -> Option<DatexValue>;
    fn as_datex_value(&self) -> DatexValue;
    fn get_type(&self) -> DatexType;
    fn add(&self, other: &dyn Value) -> Option<DatexValue>;
    fn static_type() -> DatexType
    where
        Self: Sized;

    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self
    where
        Self: Sized;

    fn as_add_assignable_mut(&mut self) -> Result<&mut dyn AddAssignable, ()> {
        Err(())
    }
}
