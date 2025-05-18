use std::{
    any::Any,
    fmt::Display,
    ops::{Add, AddAssign},
};

use serde::{Deserialize, Serialize};

use super::{
    super::core_value::CoreValue, super::datex_type::Type,
    super::typed_value::TypedValue, super::value::Value, int::I8,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text(pub String);

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl Text {
    pub fn length(&self) -> usize {
        self.0.len()
    }
    pub fn to_uppercase(&self) -> Value {
        Value::boxed(Text(self.0.to_uppercase()))
    }
    pub fn to_lowercase(&self) -> Value {
        Value::boxed(Text(self.0.to_lowercase()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl Text {
    pub fn reverse(&mut self) {
        let reversed = self.0.chars().rev().collect::<String>();
        self.0 = reversed;
    }
}

impl CoreValue for Text {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: Type) -> Option<Value> {
        match target {
            Type::Text => Some(self.as_datex_value()),
            Type::I8 => self.0.parse::<i8>().ok().map(|v| Value::boxed(I8(v))),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(self.clone())
    }

    fn static_type() -> Type {
        Type::Text
    }

    fn get_type(&self) -> Type {
        Self::static_type()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self
    where
        Self: Sized,
    {
        let s = String::from_utf8_lossy(bytes).to_string();
        Text(s)
    }
}

/// The froms are used for this magic. This will automatically convert
/// the Rust types to Text when using the += operator.
/// ```
/// # use datex_core::datex_values_new::typed_datex_value::TypedDatexValue;

/// let mut a = TypedDatexValue::from("Hello");
/// a += " World";
/// ``
impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text(s.to_string())
    }
}

impl From<i8> for Text {
    fn from(n: i8) -> Self {
        Text(n.to_string())
    }
}

impl From<String> for TypedValue<Text> {
    fn from(v: String) -> Self {
        TypedValue(Text(v))
    }
}
impl From<&str> for TypedValue<Text> {
    fn from(v: &str) -> Self {
        TypedValue(Text(v.to_string()))
    }
}

/// Might panic when the DatexValue in the assignment can not be cast to Text
impl AddAssign<Value> for TypedValue<Text> {
    fn add_assign(&mut self, rhs: Value) {
        self.0 += rhs.try_cast_to_value().unwrap_or_else(|_| {
            panic!("Cannot add DatexValue to Text");
        });
    }
}

/// Will never panic, since both TypedDatexValue and Text
impl AddAssign<TypedValue<Text>> for TypedValue<Text> {
    fn add_assign(&mut self, rhs: TypedValue<Text>) {
        self.0 += rhs.0;
    }
}

/// Allow TypedDatexValue<Text> += TypedDatexValue<PrimitiveI8>
/// This can never panic since the Text::from from i8 will always succeed
/// (#1)
impl AddAssign<TypedValue<I8>> for TypedValue<Text> {
    fn add_assign(&mut self, rhs: TypedValue<I8>) {
        self.0 += rhs.cast_to_value()
    }
}

/// Allow TypedDatexValue<Text> += String and TypedDatexValue<Text> += &str
/// This can never panic since the Text::from from string will always succeed
impl AddAssign<Text> for Text {
    fn add_assign(&mut self, rhs: Text) {
        self.0 += &rhs.0;
    }
}
/// Allow TypedDatexValue<Text> += String and TypedDatexValue<Text> += &str
/// This can never panic since the Text::from from string will always succeed
/// (#2)
impl<T> AddAssign<T> for TypedValue<Text>
where
    Text: AddAssign<Text> + From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs.into();
    }
}

impl From<I8> for Value {
    fn from(n: I8) -> Self {
        Value::boxed(Text(n.0.to_string()))
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::boxed(Text(s.to_string()))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::boxed(Text(s))
    }
}

impl PartialEq<&str> for TypedValue<Text> {
    fn eq(&self, other: &&str) -> bool {
        self.inner().as_str() == *other
    }
}

impl PartialEq<TypedValue<Text>> for &str {
    fn eq(&self, other: &TypedValue<Text>) -> bool {
        *self == other.inner().as_str()
    }
}

impl Add for Text {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Text(self.0 + &rhs.0)
    }
}
