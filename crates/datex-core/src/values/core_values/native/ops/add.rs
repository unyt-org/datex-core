use crate::{
    prelude::*,
    values::core_values::native::{DatexNative, NativeCoreValue},
};
use core::ops::Add;

impl Add for &NativeCoreValue {
    type Output = Option<NativeCoreValue>;

    fn add(self, rhs: Self) -> Self::Output {
        // println!("lhs: {}", self.value.type_name());
        // println!("rhs: {}", rhs.value.type_name());

        let value = self.value.add_native(&*rhs.value)?;
        Some(NativeCoreValue { value })
    }
}

impl Add for NativeCoreValue {
    type Output = Option<NativeCoreValue>;

    fn add(self, rhs: Self) -> Self::Output {
        (&self) + (&rhs)
    }
}

pub fn add_native_impl_option<T>(
    lhs: &T,
    rhs: &dyn DatexNative,
) -> Option<Box<dyn DatexNative>>
where
    T: DatexNative + 'static,
    for<'a> &'a T: Add<&'a T, Output = Option<T>>,
{
    let rhs = rhs.as_any().downcast_ref::<T>()?;
    let result = (lhs + rhs)?;
    Some(Box::new(result))
}

pub fn add_native_impl<T>(
    lhs: &T,
    rhs: &dyn DatexNative,
) -> Option<Box<dyn DatexNative>>
where
    T: DatexNative + 'static,
    for<'a> &'a T: Add<&'a T, Output = T>,
{
    let rhs = rhs.as_any().downcast_ref::<T>()?;
    let result = lhs + rhs;
    Some(Box::new(result))
}
