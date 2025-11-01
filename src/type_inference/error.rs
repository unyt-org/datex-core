use std::{fmt::Display, ops::Range};

use crate::{
    ast::structs::operator::binary::ArithmeticOperator,
    compiler::error::ErrorCollector, types::type_container::TypeContainer,
};

#[derive(Debug, Clone)]
pub enum TypeError {
    MismatchedOperands(ArithmeticOperator, TypeContainer, TypeContainer),

    // can not assign value to variable of different type
    AssignmentTypeMismatch {
        annotated_type: TypeContainer,
        assigned_type: TypeContainer,
    },
}

impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
