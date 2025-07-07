use std::{fmt::Display, ops::Not};

use crate::values::{
    traits::structural_eq::StructuralEq,
    value_container::{ValueContainer, ValueError},
};

use super::super::core_value_trait::CoreValueTrait;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Boolean(pub bool);

impl Boolean {
    pub fn as_bool(&self) -> bool {
        self.0
    }
}
impl Boolean {
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }
    pub fn is_true(&self) -> bool {
        self.0
    }
    pub fn is_false(&self) -> bool {
        !self.0
    }
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
    pub fn as_str(&self) -> &str {
        if self.0 {
            "true"
        } else {
            "false"
        }
    }
}

impl Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CoreValueTrait for Boolean {}

impl StructuralEq for Boolean {
    fn structural_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl From<bool> for Boolean {
    fn from(v: bool) -> Self {
        Boolean(v)
    }
}

impl Not for Boolean {
    type Output = Boolean;

    fn not(self) -> Self::Output {
        Boolean(!self.0)
    }
}
// new into
impl<T: Into<ValueContainer>> TryFrom<Option<T>> for Boolean {
    type Error = ValueError;
    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        match value {
            Some(v) => {
                let boolean: ValueContainer = v.into();
                boolean
                    .to_value()
                    .borrow()
                    .cast_to_bool()
                    .ok_or(ValueError::TypeConversionError)
            }
            None => Err(ValueError::IsVoid),
        }
    }
}
