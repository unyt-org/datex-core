use std::{any::Any, fmt::Display, ops::AddAssign};

use log::info;

use super::{
    datex_type::DatexType,
    datex_value::{AddAssignable, DatexValue, Value},
    primitive::PrimitiveI8,
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
            DatexType::PrimitiveI8 => self
                .0
                .parse::<i8>()
                .ok()
                .map(|v| DatexValue::boxed(PrimitiveI8(v))),
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
        info!("Adding {} to {}", self, other);
        // safe cast
        None
    }
}
// impl AddAssign<&str> for Text {
//     fn add_assign(&mut self, rhs: &str) {
//         self.0 += rhs;
//     }
// }

// impl AddAssign<Text> for Text {
//     fn add_assign(&mut self, rhs: Text) {
//         self.0 += &rhs.0;
//     }
// }
impl AddAssign<Text> for Text {
    fn add_assign(&mut self, rhs: Text) {
        self.0 += &rhs.0;
    }
}
