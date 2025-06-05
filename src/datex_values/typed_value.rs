use std::{
    fmt::Display,
    ops::{Add, DerefMut},
};

use super::{
    core_value::{try_cast_to_value, CoreValue},
    datex_type::CoreValueType,
    value::Value,
};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct TypedValue<T: CoreValue>(pub T);

impl<T: CoreValue + 'static> TypedValue<T> {
    pub fn into_erased(self) -> Value {
        Value::boxed(self.0)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn get_type(&self) -> CoreValueType {
        self.0.get_type()
    }
    pub fn try_cast_to_value<X: CoreValue + Clone + 'static>(
        &self,
    ) -> Result<X, ()> {
        try_cast_to_value(self.inner())
    }
    pub fn cast_to_value<X: CoreValue + Clone + 'static>(&self) -> X {
        self.try_cast_to_value().expect("Cast failed")
    }
}

impl<T> Add for TypedValue<T>
where
    T: CoreValue + Add<Output = T> + Clone,
{
    type Output = TypedValue<T>;

    fn add(self, rhs: Self) -> Self::Output {
        TypedValue(self.0 + rhs.0)
    }
}

impl<T: CoreValue + Display> Display for TypedValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: CoreValue> Deref for TypedValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: CoreValue> DerefMut for TypedValue<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct TypeMismatchError {
    pub expected: CoreValueType,
    pub found: CoreValueType,
}

impl<T: CoreValue + Clone + 'static> TryFrom<Value> for TypedValue<T> {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value
            .try_cast_to_typed::<T>()
            .map_err(|_| TypeMismatchError {
                expected: T::static_type(),
                found: value.get_type(),
            })
    }
}
impl<T: CoreValue + PartialEq + Clone + 'static> PartialEq<Value>
    for TypedValue<T>
{
    fn eq(&self, other: &Value) -> bool {
        if let Ok(casted) = other.clone().try_cast_to_typed::<T>() {
            self.0 == casted.0
        } else {
            false
        }
    }
}

impl<T> PartialEq for TypedValue<T>
where
    T: PartialEq + CoreValue,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: CoreValue> TypedValue<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}
