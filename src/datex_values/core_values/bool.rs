use std::{fmt::Display, ops::Not};

use serde::{Deserialize, Serialize};

use super::{
    super::core_value::CoreValue, super::datex_type::Type,
    super::typed_value::TypedValue, super::value::Value, text::Text,
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

impl CoreValue for Bool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: Type) -> Option<Value> {
        match target {
            Type::Bool => Some(self.as_datex_value()),
            Type::Text => Some(Value::boxed(Text(self.0.to_string()))),
            Type::I8 => Some(Value::boxed(Bool(self.0))),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(self.clone())
    }

    fn static_type() -> Type {
        Type::Bool
    }

    fn get_type(&self) -> Type {
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

impl From<Bool> for TypedValue<Bool> {
    fn from(p: Bool) -> Self {
        TypedValue(p)
    }
}

impl From<bool> for TypedValue<Bool> {
    fn from(v: bool) -> Self {
        TypedValue(Bool(v))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::boxed(Bool(v))
    }
}
impl PartialEq<bool> for TypedValue<Bool> {
    fn eq(&self, other: &bool) -> bool {
        self.inner().0 == *other
    }
}

impl PartialEq<TypedValue<Bool>> for bool {
    fn eq(&self, other: &TypedValue<Bool>) -> bool {
        *self == other.inner().0
    }
}
impl Not for TypedValue<Bool> {
    type Output = TypedValue<Bool>;

    fn not(self) -> Self::Output {
        TypedValue::from(!self.inner().0)
    }
}
