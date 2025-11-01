use crate::traits::identity::Identity;
use crate::traits::structural_eq::StructuralEq;
use crate::types::type_container::TypeContainer;
use core::cell::RefCell;

use super::value::Value;
use crate::runtime::execution::ExecutionError;
use crate::serde::deserializer::DatexDeserializer;
use crate::traits::apply::Apply;
use crate::traits::value_eq::ValueEq;
use datex_core::references::reference::Reference;
use serde::{Deserialize, Serialize};
use core::fmt::Display;
use crate::stdlib::hash::{Hash, Hasher};
use core::ops::{Add, Neg, Sub};
use crate::stdlib::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum ValueError {
    IsVoid,
    InvalidOperation,
    IntegerOverflow,
    TypeConversionError,
}

impl Display for ValueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValueError::IsVoid => core::write!(f, "Value is void"),
            ValueError::InvalidOperation => {
                core::write!(f, "Invalid operation on value")
            }
            ValueError::TypeConversionError => {
                core::write!(f, "Type conversion error")
            }
            ValueError::IntegerOverflow => {
                core::write!(f, "Integer overflow occurred")
            }
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub enum ValueContainer {
    Value(Value),
    Reference(Reference),
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
    fn hash<H: Hasher>(&self, state: &mut H) {
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
                a.structural_eq(&b.collapse_to_value().borrow())
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
                a.value_eq(&b.collapse_to_value().borrow())
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValueContainer::Value(value) => core::write!(f, "{value}"),
            // TODO #118: only simple temporary way to distinguish between Value and Pointer
            ValueContainer::Reference(reference) => {
                core::write!(f, "&({})", reference.collapse_to_value().borrow())
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
            ValueContainer::Reference(pointer) => pointer.collapse_to_value(),
        }
    }

    pub fn is_type(&self) -> bool {
        match self {
            ValueContainer::Value(value) => value.is_type(),
            ValueContainer::Reference(reference) => reference.is_type(),
        }
    }

    /// Returns the allowed type of the value container
    pub fn allowed_type(&self) -> TypeContainer {
        match self {
            // If it's a Value, return its actual type
            ValueContainer::Value(value) => value.actual_type().clone(),
            ValueContainer::Reference(reference) => {
                reference.allowed_type().clone()
            }
        }
    }

    /// Returns the actual type of the contained value, resolving references if necessary.
    pub fn actual_type(&self) -> TypeContainer {
        match self {
            ValueContainer::Value(value) => value.actual_type().clone(),
            ValueContainer::Reference(reference) => {
                reference.actual_type().clone()
            }
        }
    }

    pub fn new_value<T: Into<Value>>(value: T) -> ValueContainer {
        ValueContainer::Value(value.into())
    }

    pub fn new_reference<T: Into<Reference>>(value: T) -> ValueContainer {
        ValueContainer::Reference(value.into())
    }

    /// Returns the contained Reference if it is a Reference, otherwise returns None.
    pub fn maybe_reference(&self) -> Option<&Reference> {
        if let ValueContainer::Reference(reference) = self {
            Some(reference)
        } else {
            None
        }
    }

    /// Runs a closure with the contained Reference if it is a Reference, otherwise returns None.
    pub fn with_maybe_reference<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Reference) -> R,
    {
        if let ValueContainer::Reference(reference) = self {
            Some(f(reference))
        } else {
            None
        }
    }

    /// Returns a reference to the contained Reference, panics if it is not a Reference.
    pub fn reference_unchecked(&self) -> &Reference {
        match self {
            ValueContainer::Reference(reference) => reference,
            _ => panic!("Cannot convert ValueContainer to Reference"),
        }
    }

    /// Upgrades the ValueContainer to a ValueContainer::Reference if it is a ValueContainer::Value
    /// and if the contained value is a combined value, not a primitive value like integer, text, etc.
    pub fn upgrade_combined_value_to_reference(self) -> ValueContainer {
        match &self {
            // already a reference, no need to upgrade
            ValueContainer::Reference(_) => self,
            ValueContainer::Value(value) => {
                if value.is_collection_value() {
                    ValueContainer::new_reference(self)
                }
                // if the value is not a combined value, keep it as a ValueContainer::Value
                else {
                    self
                }
            }
        }
    }
}

impl Apply for ValueContainer {
    fn apply(
        &self,
        args: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match self {
            ValueContainer::Value(value) => todo!("#309 implement apply for Value"),
            ValueContainer::Reference(reference) => reference.apply(args),
        }
    }

    fn apply_single(
        &self,
        arg: &ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match self {
            ValueContainer::Value(value) => {
                todo!("#310 implement apply_single for Value")
            }
            ValueContainer::Reference(reference) => reference.apply_single(arg),
        }
    }
}

impl<T: Into<Value>> From<T> for ValueContainer {
    fn from(value: T) -> Self {
        ValueContainer::Value(value.into())
    }
}

impl From<TypeContainer> for ValueContainer {
    fn from(type_container: TypeContainer) -> Self {
        match type_container {
            TypeContainer::Type(type_value) => {
                ValueContainer::Value(Value::from(type_value))
            }
            TypeContainer::TypeReference(type_reference) => {
                ValueContainer::Reference(Reference::TypeReference(
                    type_reference,
                ))
            }
        }
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
                let lhs_value = lhs.collapse_to_value().borrow().clone();
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs_value + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.collapse_to_value().borrow().clone();
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
                let lhs_value = lhs.collapse_to_value().borrow().clone();
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs_value + rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs + &rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.collapse_to_value().borrow().clone();
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
                let lhs_value = lhs.collapse_to_value().borrow().clone();
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs_value - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.collapse_to_value().borrow().clone();
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
                let lhs_value = lhs.collapse_to_value().borrow().clone();
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs_value - rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Value(lhs), ValueContainer::Reference(rhs)) => {
                let rhs_value = rhs.collapse_to_value().borrow().clone();
                (lhs - &rhs_value).map(ValueContainer::Value)
            }
            (ValueContainer::Reference(lhs), ValueContainer::Value(rhs)) => {
                let lhs_value = lhs.collapse_to_value().borrow().clone();
                (&lhs_value - rhs).map(ValueContainer::Value)
            }
        }
    }
}

impl Neg for ValueContainer {
    type Output = Result<ValueContainer, ValueError>;

    fn neg(self) -> Self::Output {
        match self {
            ValueContainer::Value(value) => (-value).map(ValueContainer::Value),
            ValueContainer::Reference(reference) => {
                let value = reference.collapse_to_value().borrow().clone(); // FIXME #311: Avoid clone
                (-value).map(ValueContainer::Value)
            }
        }
    }
}
