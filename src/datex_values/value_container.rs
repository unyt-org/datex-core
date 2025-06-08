use crate::datex_values::traits::identical::Identical;
use crate::datex_values::traits::soft_eq::SoftEq;

use super::{reference::Reference, value::Value};
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, Deref, Sub};

#[derive(Debug, Clone, PartialEq)]
pub enum ValueError {
    IsVoid,
    InvalidOperation,
    IntegerOverflow,
    TypeConversionError,
}

impl Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueError::IsVoid => write!(f, "Value is void"),
            ValueError::InvalidOperation => {
                write!(f, "Invalid operation on value")
            }
            ValueError::TypeConversionError => {
                write!(f, "Type conversion error")
            }
            ValueError::IntegerOverflow => {
                write!(f, "Integer overflow occurred")
            }
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub enum ValueContainer {
    Value(Value),
    Reference(Reference),
}
impl Deref for ValueContainer {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        match self {
            ValueContainer::Value(value) => value,
            ValueContainer::Reference(pointer) => &pointer.value,
        }
    }
}

impl Hash for ValueContainer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ValueContainer::Value(value) => value.hash(state),
            ValueContainer::Reference(pointer) => pointer.value.hash(state),
        }
    }
}

impl PartialEq for ValueContainer {
    fn eq(&self, other: &Self) -> bool {
        let a = self.value();
        let b = other.value();
        a == b
    }
}

impl SoftEq for ValueContainer {
    fn soft_eq(&self, other: &Self) -> bool {
        let a = self.value();
        let b = other.value();
        a.soft_eq(b)
    }
}

impl Identical for ValueContainer {
    fn identical(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(_), ValueContainer::Value(_)) => false,
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                a.pointer_id() == b.pointer_id()
            }
            _ => false,
        }
    }
}

impl Display for ValueContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueContainer::Value(value) => write!(f, "{value}"),
            // TODO: only simple temporary way to distinguish between Value and Pointer
            ValueContainer::Reference(pointer) => {
                write!(f, "$({})", pointer.value)
            }
        }
    }
}

impl ValueContainer {
    pub fn value(&self) -> &Value {
        match self {
            ValueContainer::Value(value) => value,
            ValueContainer::Reference(pointer) => &pointer.value,
        }
    }

    pub fn into_value(self) -> Value {
        match self {
            ValueContainer::Value(value) => value,
            ValueContainer::Reference(pointer) => pointer.value,
        }
    }
}

impl<T: Into<Value>> From<T> for ValueContainer {
    fn from(value: T) -> Self {
        ValueContainer::Value(value.into())
    }
}

impl Add<ValueContainer> for ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn add(self, rhs: ValueContainer) -> Self::Output {
        let lhs = self.into_value();
        let rhs = rhs.into_value();
        (lhs + rhs).map(|v| Ok(ValueContainer::Value(v)))?
    }
}

impl Add<&ValueContainer> for &ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn add(self, rhs: &ValueContainer) -> Self::Output {
        let lhs = self.value();
        let rhs = rhs.value();
        (lhs + rhs).map(|v| Ok(ValueContainer::Value(v)))?
    }
}


impl Sub<ValueContainer> for ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn sub(self, rhs: ValueContainer) -> Self::Output {
        let lhs = self.into_value();
        let rhs = rhs.into_value();
        (lhs - rhs).map(|v| Ok(ValueContainer::Value(v)))?
    }
}

impl Sub<&ValueContainer> for &ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn sub(self, rhs: &ValueContainer) -> Self::Output {
        let lhs = self.value();
        let rhs = rhs.value();
        (lhs - rhs).map(|v| Ok(ValueContainer::Value(v)))?
    }
}