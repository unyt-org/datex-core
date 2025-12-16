use crate::traits::identity::Identity;
use crate::traits::structural_eq::StructuralEq;
use core::cell::RefCell;
use core::prelude::rust_2024::*;
use core::result::Result;

use super::value::Value;
use crate::runtime::execution::ExecutionError;
use crate::serde::deserializer::DatexDeserializer;
use crate::stdlib::boxed::Box;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::String;
use crate::traits::apply::Apply;
use crate::traits::value_eq::ValueEq;
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::r#type::Type;
use core::fmt::Display;
use core::hash::{Hash, Hasher};
use core::ops::FnOnce;
use core::ops::{Add, Neg, Sub};
use datex_core::references::reference::Reference;
use serde::Deserialize;

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValueKey<'a> {
    Text(&'a str),
    Index(i64),
    Value(&'a ValueContainer),
}

impl<'a> ValueKey<'a> {
    pub fn with_value_container<R>(
        &self,
        callback: impl FnOnce(&ValueContainer) -> R,
    ) -> R {
        match self {
            ValueKey::Value(value_container) => callback(value_container),
            ValueKey::Text(text) => {
                let value_container = ValueContainer::new_value(*text);
                callback(&value_container)
            }
            ValueKey::Index(index) => {
                let value_container = ValueContainer::new_value(*index);
                callback(&value_container)
            }
        }
    }
}

impl<'a> Display for ValueKey<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValueKey::Text(text) => core::write!(f, "{}", text),
            ValueKey::Index(index) => core::write!(f, "{}", index),
            ValueKey::Value(value_container) => {
                core::write!(f, "{}", value_container)
            }
        }
    }
}

impl<'a> From<&'a String> for ValueKey<'a> {
    fn from(text: &'a String) -> Self {
        ValueKey::Text(text)
    }
}

impl<'a> From<&'a str> for ValueKey<'a> {
    fn from(text: &'a str) -> Self {
        ValueKey::Text(text)
    }
}

impl<'a> From<i64> for ValueKey<'a> {
    fn from(index: i64) -> Self {
        ValueKey::Index(index)
    }
}

impl<'a> From<&'a ValueContainer> for ValueKey<'a> {
    fn from(value_container: &'a ValueContainer) -> Self {
        ValueKey::Value(value_container)
    }
}

impl<'a> ValueKey<'a> {
    pub fn try_as_text(&self) -> Option<&str> {
        if let ValueKey::Text(text) = self {
            Some(text)
        } else if let ValueKey::Value(ValueContainer::Value(Value {
            inner: CoreValue::Text(text),
            ..
        })) = self
        {
            Some(&text.0)
        } else {
            None
        }
    }

    pub fn try_as_index(&self) -> Option<i64> {
        if let ValueKey::Index(index) = self {
            Some(*index)
        } else if let ValueKey::Value(ValueContainer::Value(Value {
            inner: CoreValue::Integer(index),
            ..
        })) = self
        {
            index.as_i64()
        } else if let ValueKey::Value(ValueContainer::Value(Value {
            inner: CoreValue::TypedInteger(index),
            ..
        })) = self
        {
            index.as_i64()
        } else {
            None
        }
    }
}

impl<'a> From<ValueKey<'a>> for ValueContainer {
    fn from(value_key: ValueKey) -> Self {
        match value_key {
            ValueKey::Text(text) => ValueContainer::new_value(text),
            ValueKey::Index(index) => ValueContainer::new_value(index),
            ValueKey::Value(value_container) => value_container.clone(),
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

    /// Returns the actual type of the contained value, resolving references if necessary.
    pub fn actual_value_type(&self) -> TypeDefinition {
        match self {
            ValueContainer::Value(value) => value.actual_type().clone(),
            ValueContainer::Reference(reference) => {
                reference.actual_type().clone()
            }
        }
    }

    /// Returns the actual type that describes the value container (e.g. integer or &&mut integer).
    pub fn actual_container_type(&self) -> Type {
        match self {
            ValueContainer::Value(value) => {
                Type::new(*value.actual_type.clone(), None)
            }
            ValueContainer::Reference(reference) => {
                let inner_type =
                    reference.value_container().actual_container_type();
                Type::new(
                    // when nesting references, we need to keep the reference information
                    if inner_type.is_reference_type() {
                        TypeDefinition::Type(Box::new(inner_type))
                    }
                    // for simple non-ref type, we can collapse the definition
                    else {
                        inner_type.type_definition
                    },
                    Some(reference.mutability()),
                )
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
            _ => core::panic!("Cannot convert ValueContainer to Reference"),
        }
    }
}

impl Apply for ValueContainer {
    fn apply(
        &self,
        args: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match self {
            ValueContainer::Value(value) => {
                core::todo!("#309 implement apply for Value")
            }
            ValueContainer::Reference(reference) => reference.apply(args),
        }
    }

    fn apply_single(
        &self,
        arg: &ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match self {
            ValueContainer::Value(value) => {
                core::todo!("#310 implement apply_single for Value")
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
