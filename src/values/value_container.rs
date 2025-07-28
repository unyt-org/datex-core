use crate::values::traits::identity::Identity;
use crate::values::traits::structural_eq::StructuralEq;
use std::cell::RefCell;

use super::{reference::Reference, value::Value};
use crate::values::traits::value_eq::ValueEq;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, Sub};
use std::rc::Rc;
use serde::{Deserialize, Serialize};
use crate::compiler::compile_value;
use crate::values::serde::deserializer::DatexDeserializer;

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

impl Serialize for ValueContainer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct("value", &compile_value(self).unwrap())
    }
}

impl<'a> Deserialize<'a> for ValueContainer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let deserializer: &DatexDeserializer = unsafe {
            &*(&deserializer as *const D as *const DatexDeserializer)
        };
        
        Ok(deserializer.value.clone())
    }
}


impl Hash for ValueContainer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ValueContainer::Value(value) => value.hash(state),
            ValueContainer::Reference(pointer) => pointer.hash(state),
        }
    }
}

/// Partial equality for ValueContainer is identical to Hash behavior:
/// Identical references are partially equal, value-equal values are also partially equal.
/// A pointer and a value are never partially equal.
impl PartialEq for ValueContainer {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(a), ValueContainer::Value(b)) => a == b,
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                a == b
            }
            _ => false,
        }
    }
}

/// Structural equality checks the structural equality of the underlying values, collapsing
/// references to their current resolved values.
impl StructuralEq for ValueContainer {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(a), ValueContainer::Value(b)) => {
                a.structural_eq(b)
            }
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                a.structural_eq(b)
            }
            (ValueContainer::Value(a), ValueContainer::Reference(b))
            | (ValueContainer::Reference(b), ValueContainer::Value(a)) => {
                a.structural_eq(&b.borrow().current_resolved_value().borrow())
            }
        }
    }
}

/// Value equality checks the value equality of the underlying values, collapsing
/// references to their current resolved values.
impl ValueEq for ValueContainer {
    fn value_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(a), ValueContainer::Value(b)) => {
                a.value_eq(b)
            }
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                a.value_eq(b)
            }
            (ValueContainer::Value(a), ValueContainer::Reference(b))
            | (ValueContainer::Reference(b), ValueContainer::Value(a)) => {
                a.value_eq(&b.borrow().current_resolved_value().borrow())
            }
        }
    }
}

/// Identity checks only returns true if two references are identical.
/// Values are never identical to references or other values.
impl Identity for ValueContainer {
    fn identical(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueContainer::Value(_), ValueContainer::Value(_)) => false,
            (ValueContainer::Reference(a), ValueContainer::Reference(b)) => {
                a.identical(b)
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
                write!(
                    f,
                    "$({})",
                    pointer.borrow().current_resolved_value().borrow()
                )
            }
        }
    }
}

impl ValueContainer {
    pub fn to_value(&self) -> Rc<RefCell<Value>> {
        match self {
            ValueContainer::Value(value) => {
                Rc::new(RefCell::new(value.clone()))
            }
            ValueContainer::Reference(pointer) => {
                let reference = pointer.0.clone();

                reference.borrow().value_container.to_value()
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
            (
                ValueContainer::Reference(lhs),
                ValueContainer::Reference(rhs),
            ) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
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
            (
                ValueContainer::Reference(lhs),
                ValueContainer::Reference(rhs),
            ) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs + &rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
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
            (
                ValueContainer::Reference(lhs),
                ValueContainer::Reference(rhs),
            ) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
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
            (
                ValueContainer::Reference(lhs),
                ValueContainer::Reference(rhs),
            ) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs_value - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value =
                    rhs.borrow().current_resolved_value().borrow().clone();
                (lhs - &rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value =
                    lhs.borrow().current_resolved_value().borrow().clone();
                (&lhs_value - rhs).map(ValueContainer::Value)
            }
        }
    }
}
