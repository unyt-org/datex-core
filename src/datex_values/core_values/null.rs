use core::fmt;
use std::fmt::Display;

use super::super::{core_value::CoreValue, datex_type::CoreValueType, value::Value};

#[derive(Debug, Clone)]
pub struct Null;

impl CoreValue for Null {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn cast_to(&self, target: CoreValueType) -> Option<Value> {
        match target {
            CoreValueType::Null => Some(Value::boxed(Null)),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(Null)
    }

    fn get_type(&self) -> CoreValueType {
        Self::static_type()
    }

    fn static_type() -> CoreValueType {
        CoreValueType::Null
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
