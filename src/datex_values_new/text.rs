use std::{any::Any, fmt::Display};

use super::{
    datex_type::DatexType,
    datex_value::{DatexValue, Value},
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
}

impl Value for Text {
    fn as_any(&self) -> &dyn Any {
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

    fn get_type(&self) -> DatexType {
        DatexType::Text
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
