use std::fmt::Display;
use std::ops::Add;
use super::{pointer::Pointer, value::Value};

#[derive(Clone, Debug, PartialEq, Default)]
pub enum ValueContainer {
    Value(Value),
    Pointer(Pointer),
    #[default]
    Void,
}

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


// impl From<Value> for ValueContainer {
//     fn from(value: Value) -> Self {
//         ValueContainer::Value(value)
//     }
// }

impl<T: Into<Value>> From<T> for ValueContainer {
    fn from(value: T) -> Self {
        ValueContainer::Value(value.into())
    }
}


impl Add<ValueContainer> for ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn add(self, rhs: ValueContainer) -> Self::Output {
        let lhs = Value::try_from(self)?;
        let rhs = Value::try_from(rhs)?;
        (lhs + rhs)
            .map(|v| Ok(ValueContainer::Value(v)))?
    }
}

impl Add<&ValueContainer> for &ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn add(self, rhs: &ValueContainer) -> Self::Output {
        let lhs = Value::try_from(self)?;
        let rhs = Value::try_from(rhs)?;
        (lhs + rhs)
            .map(|v| Ok(ValueContainer::Value(v)))?
    }
}