use crate::{
    global::operators::binary::ArithmeticOperator, prelude::*,
    types::r#type::Type,
};
use core::fmt::Display;

#[derive(Debug, PartialEq, Eq)]
pub enum IllegalTypeError {
    MutableRef(String),
    TypeNotFound,
}

impl Display for IllegalTypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IllegalTypeError::MutableRef(val) => {
                core::write!(f, "Cannot use mutable reference as type: {}", val)
            }
            IllegalTypeError::TypeNotFound => {
                core::write!(f, "Core type not found in memory")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeError {
    Invalid,
    SubvariantNotFound(String, String),
    // only for debugging purposes
    InvalidUnboxType(Box<Type>),
    Unimplemented(String),
    MismatchedOperands(Box<MismatchedOperandsError>),
    AssignmentToImmutableReference(String),
    AssignmentToImmutableValue(String),
    AssignmentToConstant(String),
    ReferenceToNonTypeValue,
    InvalidSharedReference,
    UnsupportedApply(Box<Type>),
    UnsupportedPropertyAccess(Box<UnsupportedPropertyAccessError>),
    // can not assign value to variable of different type
    AssignmentTypeMismatch(Box<AssignmentTypeMismatchError>),
}

impl TypeError {
    pub fn subvariant_not_found(ty: String, variant: String) -> Self {
        TypeError::SubvariantNotFound(ty, variant)
    }
    pub fn invalid_unbox_type(ty: Type) -> Self {
        TypeError::InvalidUnboxType(Box::new(ty))
    }
    pub fn unimplemented(msg: String) -> Self {
        TypeError::Unimplemented(msg)
    }
    pub fn mismatched_operands(
        operator: ArithmeticOperator,
        lhs: Type,
        rhs: Type,
    ) -> Self {
        TypeError::MismatchedOperands(Box::new(MismatchedOperandsError {
            operator,
            lhs,
            rhs,
        }))
    }
    pub fn assignment_to_immutable_reference(var_name: String) -> Self {
        TypeError::AssignmentToImmutableReference(var_name)
    }
    pub fn assignment_to_immutable_value(var_name: String) -> Self {
        TypeError::AssignmentToImmutableValue(var_name)
    }
    pub fn assignment_to_constant(var_name: String) -> Self {
        TypeError::AssignmentToConstant(var_name)
    }
    pub fn reference_to_non_type_value() -> Self {
        TypeError::ReferenceToNonTypeValue
    }
    pub fn invalid_shared_reference() -> Self {
        TypeError::InvalidSharedReference
    }
    pub fn unsupported_apply(ty: Type) -> Self {
        TypeError::UnsupportedApply(Box::new(ty))
    }
    pub fn unsupported_property_access(base: Type, property: Type) -> Self {
        TypeError::UnsupportedPropertyAccess(Box::new(
            UnsupportedPropertyAccessError { base, property },
        ))
    }
    pub fn assignment_type_mismatch(expected: Type, found: Type) -> Self {
        TypeError::AssignmentTypeMismatch(Box::new(
            AssignmentTypeMismatchError { expected, found },
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssignmentTypeMismatchError {
    pub expected: Type,
    pub found: Type,
}

impl Display for AssignmentTypeMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Cannot assign {} to {}", self.found, self.expected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnsupportedPropertyAccessError {
    pub base: Type,
    pub property: Type,
}
impl Display for UnsupportedPropertyAccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Type {} does not support property access with property {}",
            self.base, self.property
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MismatchedOperandsError {
    pub operator: ArithmeticOperator,
    pub lhs: Type,
    pub rhs: Type,
}
impl Display for MismatchedOperandsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Cannot perform \"{}\" operation on {} and {}",
            self.operator, self.lhs, self.rhs
        )
    }
}

impl Display for TypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TypeError::Invalid => {
                write!(f, "Invalid type error")
            }
            TypeError::UnsupportedApply(ty) => {
                write!(f, "Cannot apply non-callable type {}", ty)
            }
            TypeError::AssignmentToImmutableValue(var_name) => {
                write!(f, "Cannot assign to immutable variable '{}'", var_name)
            }
            TypeError::AssignmentToConstant(var_name) => {
                write!(f, "Cannot assign to constant variable '{}'", var_name)
            }
            TypeError::AssignmentToImmutableReference(var_name) => {
                write!(
                    f,
                    "Cannot assign to immutable reference variable '{}'",
                    var_name
                )
            }
            TypeError::SubvariantNotFound(ty, variant) => {
                write!(
                    f,
                    "Type {} does not have a subvariant named {}",
                    ty, variant
                )
            }
            TypeError::InvalidUnboxType(ty) => {
                write!(f, "Cannot unbox value of type {}", ty)
            }
            TypeError::Unimplemented(msg) => {
                write!(f, "Unimplemented type inference case: {}", msg)
            }
            TypeError::ReferenceToNonTypeValue => {
                write!(f, "Invalid reference to non-type value")
            }
            TypeError::InvalidSharedReference => {
                write!(f, "Invalid shared reference to non-shared value")
            }
            TypeError::MismatchedOperands(err) => {
                write!(f, "{}", err)
            }
            TypeError::UnsupportedPropertyAccess(err) => {
                write!(f, "{}", err)
            }
            TypeError::AssignmentTypeMismatch(err) => {
                write!(f, "{}", err)
            }
        }
    }
}
