use std::fmt::Display;

use crate::datex_values::soft_eq::SoftEq;

pub trait CoreValueTrait: Display + Send + Sync + SoftEq {
    // fn as_any(&self) -> &dyn Any;
    // fn as_any_mut(&mut self) -> &mut dyn Any;
    // fn cast_to(&self, target: CoreValueType) -> Option<Value>;
    // fn as_datex_value(&self) -> Value;
    // fn get_type(&self) -> CoreValueType;
    // fn static_type() -> CoreValueType
    // where
    //     Self: Sized;
}
//
// pub fn try_cast_to_value<T: CoreValue + Clone + 'static>(
//     value: &impl CoreValue,
// ) -> Result<T, ()> {
//     let casted = value.cast_to(T::static_type()).ok_or(())?;
//     let casted = casted.cast_to_typed::<T>();
//     Ok(casted.into_inner())
// }
//
// pub fn try_cast_to_value_dyn<T: CoreValue + Clone + 'static>(
//     value: &dyn CoreValue,
// ) -> Result<T, ()> {
//     let casted = value.cast_to(T::static_type()).ok_or(())?;
//     let casted = casted.cast_to_typed::<T>();
//     Ok(casted.into_inner())
// }
