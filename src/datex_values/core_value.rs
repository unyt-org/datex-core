use std::any::Any;
use std::fmt::Display;

use super::datex_type::Type;
use super::value::Value;

pub trait CoreValue: Display + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn cast_to(&self, target: Type) -> Option<Value>;
    fn as_datex_value(&self) -> Value;
    fn get_type(&self) -> Type;
    fn static_type() -> Type
    where
        Self: Sized;
}

pub fn try_cast_to_value<T: CoreValue + Clone + 'static>(
    value: &impl CoreValue,
) -> Result<T, ()> {
    let casted = value.cast_to(T::static_type()).ok_or(())?;
    let casted = casted.cast_to_typed::<T>();
    Ok(casted.into_inner())
}

pub fn try_cast_to_value_dyn<T: CoreValue + Clone + 'static>(
    value: &dyn CoreValue,
) -> Result<T, ()> {
    let casted = value.cast_to(T::static_type()).ok_or(())?;
    let casted = casted.cast_to_typed::<T>();
    Ok(casted.into_inner())
}
