use core::fmt;
use std::fmt::Display;

use super::{
    datex_type::DatexType,
    datex_value::{DatexValue, Value},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Null;

impl Value for Null {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_type(&self) -> DatexType {
        DatexType::Null
    }

    fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        match target {
            DatexType::Null => Some(DatexValue::boxed(Null)),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> DatexValue {
        DatexValue::boxed(Null)
    }

    fn add(&self, _: &dyn Value) -> Option<DatexValue> {
        None
    }
}

impl Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "null")
    }
}
