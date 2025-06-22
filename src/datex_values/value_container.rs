use std::cell::RefCell;
use crate::datex_values::traits::identical::Identical;
use crate::datex_values::traits::soft_eq::SoftEq;

use super::{reference::Reference, value::Value};
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, Deref, Sub};
use std::rc::Rc;

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

impl Hash for ValueContainer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ValueContainer::Value(value) => value.hash(state),
            ValueContainer::Reference(pointer) => pointer.hash(state),
        }
    }
}

impl PartialEq for ValueContainer {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(a), ValueContainer::Value(b)) => a == b,
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                *a.borrow().current_resolved_value().borrow() ==
                    *b.borrow().current_resolved_value().borrow()
            }
            (ValueContainer::Value(a), ValueContainer::Reference(b)) |
            (ValueContainer::Reference(b), ValueContainer::Value(a)) => {
                *a == *b.borrow().current_resolved_value().borrow()
            }
        }
    }
}

impl SoftEq for ValueContainer {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(a), ValueContainer::Value(b)) => a.soft_eq(b),
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                a.borrow().current_resolved_value().borrow().soft_eq(
                    &b.borrow().current_resolved_value().borrow()
                )
            }
            (ValueContainer::Value(a), ValueContainer::Reference(b)) |
            (ValueContainer::Reference(b), ValueContainer::Value(a)) => {
                a.soft_eq(&b.borrow().current_resolved_value().borrow())
            }
        }
    }
}

impl Identical for ValueContainer {
    fn identical(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(_), ValueContainer::Value(_)) => false,
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                a == b
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
                write!(f, "$({})", pointer.borrow().current_resolved_value().borrow())
            }
        }
    }
}

impl ValueContainer {
    pub fn to_value(&self) -> Rc<RefCell<Value>> {
        match self {
            ValueContainer::Value(value) => Rc::new(RefCell::new(value.clone())),
            ValueContainer::Reference(pointer) => {
                let reference = pointer.0.clone();
                let val = reference.borrow().value_container.to_value();
                val
            }
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
        match (self, rhs) {
            (ValueContainer::Value(lhs), ValueContainer::Value(rhs)) => {
                (lhs + rhs).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Reference(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value + rhs).map(ValueContainer::Value)
            }
        }
    }
}

impl Add<&ValueContainer> for &ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn add(self, rhs: &ValueContainer) -> Self::Output {
        match (self, rhs) {
            (ValueContainer::Value(lhs), ValueContainer::Value(rhs)) => {
                (lhs + rhs).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Reference(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs + &rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                (&lhs_value + rhs).map(ValueContainer::Value)
            }
        }
    }
}


impl Sub<ValueContainer> for ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn sub(self, rhs: ValueContainer) -> Self::Output {
        match (self, rhs) {
            (ValueContainer::Value(lhs), ValueContainer::Value(rhs)) => {
                (lhs - rhs).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Reference(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value - rhs).map(ValueContainer::Value)
            }
        }
    }
}

impl Sub<&ValueContainer> for &ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn sub(self, rhs: &ValueContainer) -> Self::Output {
        match (self, rhs) {
            (ValueContainer::Value(lhs), ValueContainer::Value(rhs)) => {
                (lhs - rhs).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Reference(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.borrow().current_resolved_value().borrow().clone();
                (lhs - &rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.borrow().current_resolved_value().borrow().clone();
                (&lhs_value - rhs).map(ValueContainer::Value)
            }
        }
    }
}