use core::fmt;
use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::super::{core_value::CoreValue, datex_type::Type, value::Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Null;

impl CoreValue for Null {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn static_type() -> Type {
        Type::Null
    }

    fn get_type(&self) -> Type {
        Self::static_type()
    }

    fn cast_to(&self, target: Type) -> Option<Value> {
        match target {
            Type::Null => Some(Value::boxed(Null)),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(Null)
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
