use std::{
    fmt::Display,
    ops::{Add, DerefMut},
};

use super::{datex_type::DatexType, datex_value::DatexValue, value::Value};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct TypedDatexValue<T: Value>(pub T);

impl<T: Value + 'static> TypedDatexValue<T> {
    pub fn into_erased(self) -> DatexValue {
        DatexValue::boxed(self.0)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn get_type(&self) -> DatexType {
        self.0.get_type()
    }
}

impl<T> Add for TypedDatexValue<T>
where
    T: Value + Add<Output = T> + Clone,
{
    type Output = TypedDatexValue<T>;

    fn add(self, rhs: Self) -> Self::Output {
        TypedDatexValue(self.0 + rhs.0)
    }
}

impl<T: Value + Display> Display for TypedDatexValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Value> Deref for TypedDatexValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Value> DerefMut for TypedDatexValue<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct TypeMismatchError {
    pub expected: DatexType,
    pub found: DatexType,
}

impl<T: Value + Clone + 'static> TryFrom<DatexValue> for TypedDatexValue<T> {
    type Error = TypeMismatchError;

    fn try_from(value: DatexValue) -> Result<Self, Self::Error> {
        value
            .try_cast_to_typed::<T>()
            .map_err(|_| TypeMismatchError {
                expected: T::static_type(),
                found: value.get_type(),
            })
    }
}
impl<T: Value + PartialEq + Clone + 'static> PartialEq<DatexValue>
    for TypedDatexValue<T>
{
    fn eq(&self, other: &DatexValue) -> bool {
        if let Ok(casted) = other.clone().try_cast_to_typed::<T>() {
            self.0 == casted.0
        } else {
            false
        }
    }
}

// impl<T: Value + PartialEq + Clone + 'static> PartialEq<TypedDatexValue<T>>
//     for DatexValue
// {
//     fn eq(&self, other: &TypedDatexValue<T>) -> bool {
//         other == self
//     }
// }
impl<T> PartialEq for TypedDatexValue<T>
where
    T: PartialEq + Value,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
