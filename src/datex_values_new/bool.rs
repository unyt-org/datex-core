use std::{fmt::Display, ops::Not};

use serde::{Deserialize, Serialize};

use super::{
    datex_type::DatexType, datex_value::DatexValue, text::Text,
    typed_datex_value::TypedDatexValue, value::Value,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bool(pub bool);

impl Bool {
    pub fn as_bool(&self) -> bool {
        self.0
    }
}
impl Bool {
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }
}

impl Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Value for Bool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        match target {
            DatexType::Bool => Some(self.as_datex_value()),
            DatexType::Text => {
                Some(DatexValue::boxed(Text(self.0.to_string())))
            }
            DatexType::I8 => Some(DatexValue::boxed(Bool(self.0))),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> DatexValue {
        DatexValue::boxed(self.clone())
    }

    fn static_type() -> DatexType {
        DatexType::Bool
    }

    fn get_type(&self) -> DatexType {
        Self::static_type()
    }
    fn to_bytes(&self) -> Vec<u8> {
        vec![if self.0 { 1 } else { 0 }]
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut arr = [0; 1];
        arr.copy_from_slice(&bytes[0..1]);
        Bool(arr[0] != 0)
    }
}

impl From<Bool> for TypedDatexValue<Bool> {
    fn from(p: Bool) -> Self {
        TypedDatexValue(p)
    }
}

impl From<bool> for TypedDatexValue<Bool> {
    fn from(v: bool) -> Self {
        TypedDatexValue(Bool(v))
    }
}

impl From<bool> for DatexValue {
    fn from(v: bool) -> Self {
        DatexValue::boxed(Bool(v))
    }
}
impl PartialEq<bool> for TypedDatexValue<Bool> {
    fn eq(&self, other: &bool) -> bool {
        self.inner().0 == *other
    }
}

impl PartialEq<TypedDatexValue<Bool>> for bool {
    fn eq(&self, other: &TypedDatexValue<Bool>) -> bool {
        *self == other.inner().0
    }
}
impl Not for TypedDatexValue<Bool> {
    type Output = TypedDatexValue<Bool>;

    fn not(self) -> Self::Output {
        TypedDatexValue::from(!self.inner().0)
    }
}
