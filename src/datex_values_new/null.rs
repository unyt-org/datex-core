use core::fmt;
use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::{datex_type::DatexType, datex_value::DatexValue, value::Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Null;

impl Value for Null {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn static_type() -> DatexType {
        DatexType::Null
    }

    fn get_type(&self) -> DatexType {
        Self::static_type()
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
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }
    fn from_bytes(_: &[u8]) -> Self {
        Null
    }
}

impl Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "null")
    }
}
impl PartialEq for Null {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
