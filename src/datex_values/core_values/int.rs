use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use serde::{Deserialize, Serialize};

use super::{
    super::core_value::CoreValue, super::datex_type::Type,
    super::typed_value::TypedValue, super::value::Value, text::Text,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct I8(pub i8);

impl Display for I8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CoreValue for I8 {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: Type) -> Option<Value> {
        match target {
            Type::I8 => Some(self.as_datex_value()),
            Type::Text => Some(Value::boxed(Text(self.0.to_string()))),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(self.clone())
    }

    fn static_type() -> Type {
        Type::I8
    }

    fn get_type(&self) -> Type {
        Self::static_type()
    }
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut arr = [0; 1];
        arr.copy_from_slice(&bytes[0..1]);
        I8(i8::from_le_bytes(arr))
    }
}

impl Add for I8 {
    type Output = I8;

    fn add(self, rhs: Self) -> Self::Output {
        I8(self.0 + rhs.0)
    }
}

impl AddAssign for I8 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl From<I8> for TypedValue<I8> {
    fn from(p: I8) -> Self {
        TypedValue(p)
    }
}

impl From<i8> for TypedValue<I8> {
    fn from(v: i8) -> Self {
        TypedValue(I8(v))
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::boxed(I8(v))
    }
}
impl PartialEq<i8> for TypedValue<I8> {
    fn eq(&self, other: &i8) -> bool {
        self.inner().0 == *other
    }
}

impl PartialEq<TypedValue<I8>> for i8 {
    fn eq(&self, other: &TypedValue<I8>) -> bool {
        *self == other.inner().0
    }
}
