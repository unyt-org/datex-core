use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use super::{
    datex_type::DatexType,
    datex_value::{DatexValue, Value},
    primitive::PrimitiveI8,
    text::Text,
};

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

impl From<PrimitiveI8> for TypedDatexValue<PrimitiveI8> {
    fn from(p: PrimitiveI8) -> Self {
        TypedDatexValue(p)
    }
}

impl From<i8> for TypedDatexValue<PrimitiveI8> {
    fn from(v: i8) -> Self {
        TypedDatexValue(PrimitiveI8(v))
    }
}

impl From<String> for TypedDatexValue<Text> {
    fn from(v: String) -> Self {
        TypedDatexValue(Text(v))
    }
}
impl From<&str> for TypedDatexValue<Text> {
    fn from(v: &str) -> Self {
        TypedDatexValue(Text(v.to_string()))
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

use std::ops::Deref;

impl<T: Value> Deref for TypedDatexValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// impl AddAssign<&str> for TypedDatexValue<Text> {
//     fn add_assign(&mut self, rhs: &str) {
//         self.0 += rhs;
//     }
// }

// impl AddAssign<Text> for TypedDatexValue<Text> {
//     fn add_assign(&mut self, rhs: Text) {
//         self.0 += rhs;
//     }
// }

// impl AddAssign<DatexValue> for TypedDatexValue<Text> {
//     fn add_assign(&mut self, rhs: DatexValue) {
//         if let Some(casted) = rhs.cast_to_typed::<Text>() {
//             self.0 += casted.0;
//         } else {
//             panic!("Cannot cast DatexValue to Text");
//         }
//     }
// }

impl AddAssign<DatexValue> for TypedDatexValue<Text> {
    fn add_assign(&mut self, rhs: DatexValue) {
        if let Some(casted) = rhs.cast_to_typed::<Text>() {
            self.0 += casted.0;
        } else {
            panic!("Cannot cast DatexValue to Text");
        }
    }
}

impl<T> AddAssign<T> for TypedDatexValue<Text>
where
    Text: AddAssign<Text> + From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.0 += Text::from(rhs);
    }
}
