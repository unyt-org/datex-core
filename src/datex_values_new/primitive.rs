use std::fmt::Display;

use super::{
    datex_type::DatexType,
    datex_value::{DatexValue, Value},
    text::Text,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveI8(pub i8);

impl Display for PrimitiveI8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Value for PrimitiveI8 {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        match target {
            DatexType::PrimitiveI8 => Some(self.as_datex_value()),
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
        match other.cast_to(DatexType::PrimitiveI8) {
            Some(DatexValue(val)) => {
                if let Some(other_i8) =
                    val.as_ref().as_any().downcast_ref::<PrimitiveI8>()
                {
                    Some(DatexValue::boxed(PrimitiveI8(self.0 + other_i8.0)))
                } else {
                    None
                }
            }
            _ => {
                let self_str = self.cast_to(DatexType::Text)?;
                self_str.0.add(other)
            }
        }
    }

    fn get_type(&self) -> DatexType {
        DatexType::PrimitiveI8
    }
}
