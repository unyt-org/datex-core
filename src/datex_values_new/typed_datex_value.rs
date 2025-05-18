use std::{
    fmt::Display,
    ops::{Add, DerefMut},
};

use super::{
    datex_type::DatexType,
    datex_value::{DatexValue, Value},
};
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
