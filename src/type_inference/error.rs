use core::{fmt::Display, ops::Range};

use crate::values::core_values::r#type::Type;
use crate::{
    compiler::error::ErrorCollector,
    global::operators::binary::ArithmeticOperator,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeError {
    SubvariantNotFound(String, String),
    // only for debugging purposes
    InvalidDerefType(Type),
    Unimplemented(String),
    MismatchedOperands(ArithmeticOperator, Type, Type),
    AssignmentToImmutableReference(String),
    AssignmentToImmutableValue(String),
    AssignmentToConstant(String),

    // can not assign value to variable of different type
    AssignmentTypeMismatch {
        annotated_type: Type,
        assigned_type: Type,
    },
}

impl Display for TypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
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
            TypeError::InvalidDerefType(ty) => {
                write!(f, "Cannot dereference value of type {}", ty)
            }
            TypeError::Unimplemented(msg) => {
                write!(f, "Unimplemented type inference case: {}", msg)
            }
            TypeError::MismatchedOperands(op, lhs, rhs) => {
                write!(
                    f,
                    "Cannot perform \"{}\" operation on {} and {}",
                    op, lhs, rhs
                )
            }
            TypeError::AssignmentTypeMismatch {
                annotated_type,
                assigned_type,
            } => {
                write!(
                    f,
                    "Cannot assign {} to {}",
                    assigned_type, annotated_type
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct SpannedTypeError {
    pub error: TypeError,
    pub span: Option<Range<usize>>,
}

impl SpannedTypeError {
    pub fn new_with_span(
        error: TypeError,
        span: Range<usize>,
    ) -> SpannedTypeError {
        SpannedTypeError {
            error,
            span: Some(span),
        }
    }
}

impl From<TypeError> for SpannedTypeError {
    fn from(value: TypeError) -> Self {
        SpannedTypeError {
            error: value,
            span: None,
        }
    }
}

#[derive(Debug)]
pub struct DetailedTypeErrors {
    pub errors: Vec<SpannedTypeError>,
}

impl ErrorCollector<SpannedTypeError> for DetailedTypeErrors {
    fn record_error(&mut self, error: SpannedTypeError) {
        self.errors.push(error);
    }
}

impl DetailedTypeErrors {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[derive(Debug)]
pub enum SimpleOrDetailedTypeError {
    Simple(SpannedTypeError),
    Detailed(DetailedTypeErrors),
}

impl From<SpannedTypeError> for SimpleOrDetailedTypeError {
    fn from(value: SpannedTypeError) -> Self {
        SimpleOrDetailedTypeError::Simple(value)
    }
}

impl From<DetailedTypeErrors> for SimpleOrDetailedTypeError {
    fn from(value: DetailedTypeErrors) -> Self {
        SimpleOrDetailedTypeError::Detailed(value)
    }
}
