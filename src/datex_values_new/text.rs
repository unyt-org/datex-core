use std::{any::Any, fmt::Display, ops::AddAssign};

use super::{
    datex_type::DatexType,
    datex_value::DatexValue,
    int::I8,
    typed_datex_value::TypedDatexValue,
    value::{AddAssignable, Value},
};

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn to_uppercase(&self) -> DatexValue {
        DatexValue::boxed(Text(self.0.to_uppercase()))
    }
    pub fn to_lowercase(&self) -> DatexValue {
        DatexValue::boxed(Text(self.0.to_lowercase()))
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

impl Value for Text {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        match target {
            DatexType::Text => Some(self.as_datex_value()),
            DatexType::I8 => {
                self.0.parse::<i8>().ok().map(|v| DatexValue::boxed(I8(v)))
            }
            _ => None,
        }
    }

    fn as_datex_value(&self) -> DatexValue {
        DatexValue::boxed(self.clone())
    }

    fn static_type() -> DatexType {
        DatexType::Text
    }
    fn as_add_assignable_mut(&mut self) -> Result<&mut dyn AddAssignable, ()> {
        Ok(self)
    }

    fn get_type(&self) -> DatexType {
        Self::static_type()
    }
    fn add(&self, other: &dyn Value) -> Option<DatexValue> {
        let other_casted = other.cast_to(DatexType::Text)?;
        let other_text =
            other_casted.0.as_ref().as_any().downcast_ref::<Text>()?;
        Some(DatexValue::boxed(Text(format!(
            "{}{}",
            self.0, other_text.0
        ))))
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

impl AddAssignable for Text {
    fn add_assign_boxed(&mut self, other: &dyn Value) -> Option<()> {
        let rhs_text = other.cast_to(DatexType::Text)?;
        let rhs_text = rhs_text.0.as_any().downcast_ref::<Text>()?;
        self.0 += &rhs_text.0;
        Some(())
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

/// Might panic when the DatexValue in the assignment can not be cast to Text
impl AddAssign<DatexValue> for TypedDatexValue<Text> {
    fn add_assign(&mut self, rhs: DatexValue) {
        self.add_assign_boxed(rhs.0.as_ref()).or_else(|| {
            panic!("Cannot add DatexValue to Text");
        });
    }
}

/// Will never panic, since both TypedDatexValue and Text
impl AddAssign<TypedDatexValue<Text>> for TypedDatexValue<Text> {
    fn add_assign(&mut self, rhs: TypedDatexValue<Text>) {
        self.add_assign_boxed(rhs.into_erased().0.as_ref());
    }
}

/// Allow TypedDatexValue<Text> += TypedDatexValue<PrimitiveI8>
/// This can never panic since the Text::from from i8 will always succeed
/// (#1)
impl AddAssign<TypedDatexValue<I8>> for TypedDatexValue<Text> {
    fn add_assign(&mut self, rhs: TypedDatexValue<I8>) {
        self.add_assign_boxed(rhs.into_erased().0.as_ref());
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
impl<T> AddAssign<T> for TypedDatexValue<Text>
where
    Text: AddAssign<Text> + From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.add_assign_boxed(&Text::from(rhs));
    }
}

impl From<&str> for DatexValue {
    fn from(s: &str) -> Self {
        DatexValue::boxed(Text(s.to_string()))
    }
}

impl From<String> for DatexValue {
    fn from(s: String) -> Self {
        DatexValue::boxed(Text(s))
    }
}

impl PartialEq<&str> for TypedDatexValue<Text> {
    fn eq(&self, other: &&str) -> bool {
        self.inner().as_str() == *other
    }
}

impl PartialEq<TypedDatexValue<Text>> for &str {
    fn eq(&self, other: &TypedDatexValue<Text>) -> bool {
        *self == other.inner().as_str()
    }
}
