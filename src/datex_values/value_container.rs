use std::fmt::Display;
use std::ops::Add;
use super::{pointer::Pointer, value::Value};

#[derive(Debug, Clone, PartialEq)]
pub enum ValueError {
    IsVoid,
    InvalidOperation,
    TypeConversionError,
}

impl Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueError::IsVoid => write!(f, "Value is void"),
            ValueError::InvalidOperation => write!(f, "Invalid operation on value"),
            ValueError::TypeConversionError => write!(f, "Type conversion error"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueContainer {
    Value(Value),
    Pointer(Pointer),
}

impl Display for ValueContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueContainer::Value(value) => write!(f, "{value}"),
            // TODO: only simple temporary way to distinguish between Value and Pointer
            ValueContainer::Pointer(pointer) => write!(f, "$({})", pointer.value),
        }
    }
}

impl ValueContainer {
    pub fn get_value(&self) -> &Value {
        match self {
            ValueContainer::Value(value) => value,
            ValueContainer::Pointer(pointer) => &pointer.value,
        }
    }
    
    pub fn into_value(self) -> Value {
        match self {
            ValueContainer::Value(value) => value,
            ValueContainer::Pointer(pointer) => pointer.value,
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
        (lhs + rhs)
            .map(|v| Ok(ValueContainer::Value(v)))?
    }
}

impl Add<&ValueContainer> for &ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn add(self, rhs: &ValueContainer) -> Self::Output {
        let lhs = self.get_value();
        let rhs = rhs.get_value();
        (lhs + rhs)
            .map(|v| Ok(ValueContainer::Value(v)))?
    }
}