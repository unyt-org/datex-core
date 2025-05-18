use std::{fmt::Display, ops::Add};

use serde::{Deserialize, Serialize};

use super::{
    datex_type::DatexType, datex_value::DatexValue, text::Text,
    typed_datex_value::TypedDatexValue, value::Value,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct I8(pub i8);

impl Display for I8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Value for I8 {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        match target {
            DatexType::I8 => Some(self.as_datex_value()),
            DatexType::Text => {
                Some(DatexValue::boxed(Text(self.0.to_string())))
            }
            _ => None,
        }
    }

    fn as_datex_value(&self) -> DatexValue {
        DatexValue::boxed(self.clone())
    }

    fn add(&self, other: &dyn Value) -> Option<DatexValue> {
        match other.cast_to(DatexType::I8) {
            Some(DatexValue(val)) => val
                .as_ref()
                .as_any()
                .downcast_ref::<I8>()
                .map(|other_i8| DatexValue::boxed(I8(self.0 + other_i8.0))),
            _ => {
                let self_str = self.cast_to(DatexType::Text)?;
                self_str.0.add(other)
            }
        }
    }

    fn static_type() -> DatexType {
        DatexType::I8
    }

    fn get_type(&self) -> DatexType {
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

impl From<I8> for TypedDatexValue<I8> {
    fn from(p: I8) -> Self {
        TypedDatexValue(p)
    }
}

impl From<i8> for TypedDatexValue<I8> {
    fn from(v: i8) -> Self {
        TypedDatexValue(I8(v))
    }
}

impl From<i8> for DatexValue {
    fn from(v: i8) -> Self {
        DatexValue::boxed(I8(v))
    }
}
impl PartialEq<i8> for TypedDatexValue<I8> {
    fn eq(&self, other: &i8) -> bool {
        self.inner().0 == *other
    }
}

impl PartialEq<TypedDatexValue<I8>> for i8 {
    fn eq(&self, other: &TypedDatexValue<I8>) -> bool {
        *self == other.inner().0
    }
}
