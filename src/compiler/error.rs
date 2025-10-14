use crate::ast::error::error::ParseError;
use crate::ast::tree::DatexExpression;
use std::fmt::Display;
use std::ops::Range;
use chumsky::span::SimpleSpan;
use crate::compiler::type_inference::TypeError;

#[derive(Debug)]
pub enum CompilerError {
    UnexpectedTerm(Box<DatexExpression>),
    ParseErrors(Vec<ParseError>),
    SerializationError(binrw::Error),
    BigDecimalOutOfBoundsError,
    IntegerOutOfBoundsError,
    InvalidPlaceholderCount,
    NonStaticValue,
    UndeclaredVariable(String),
    InvalidRedeclaration(String),
    SubvariantNotFound(String, String),
    ScopePopError,
    InvalidSlotName(String),
    AssignmentToConst(String),
    AssignmentToImmutableReference(String),
    AssignmentToImmutableValue(String),
    OnceScopeUsedMultipleTimes,
    TypeError(TypeError),
    Spanned(Box<CompilerError>, Range<usize>),
    Multiple(Vec<CompilerError>),
}

impl CompilerError {
    /// Wraps the error in a CompilerError::Spanned with the given span
    pub fn spanned_from_simple_span(self, span: SimpleSpan) -> Self {
        CompilerError::Spanned(Box::new(self), span.start..span.end)
    }
    /// Wraps the error in a CompilerError::Spanned with the given range
    pub fn spanned(self, range: Range<usize>) -> Self {
        CompilerError::Spanned(Box::new(self), range)
    }

    /// Creates a CompilerError::Multiple from a vector of errors
    pub fn multiple(errors: Vec<CompilerError>) -> Self {
        CompilerError::Multiple(errors)
    }

    /// Appends multiple errors into one CompilerError::Multiple
    pub fn append(self, mut other: Vec<CompilerError>) -> Self {
        match self {
            CompilerError::Multiple(mut errs) => {
                errs.append(&mut other);
                CompilerError::Multiple(errs)
            }
            err => {
                other.insert(0, err);
                CompilerError::Multiple(other)
            }
        }
    }
}

impl From<Vec<ParseError>> for CompilerError {
    fn from(value: Vec<ParseError>) -> Self {
        CompilerError::ParseErrors(value)
    }
}

impl From<TypeError> for CompilerError {
    fn from(value: TypeError) -> Self {
        CompilerError::TypeError(value)
    }
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::InvalidRedeclaration(name) => {
                write!(f, "Invalid redeclaration of {name}")
            }
            CompilerError::UnexpectedTerm(rule) => {
                write!(f, "Unexpected term: {rule:?}")
            }
            CompilerError::ParseErrors(error) => {
                for e in error {
                    writeln!(f, "{}", e.message())?;
                }
                Ok(())
            }
            CompilerError::SubvariantNotFound(name, variant) => {
                write!(f, "Subvariant {variant} does not exist for {name}")
            }
            CompilerError::SerializationError(error) => {
                write!(f, "Serialization error: {error}")
            }
            CompilerError::BigDecimalOutOfBoundsError => {
                write!(f, "BigDecimal out of bounds error")
            }
            CompilerError::IntegerOutOfBoundsError => {
                write!(f, "Integer out of bounds error")
            }
            CompilerError::InvalidPlaceholderCount => {
                write!(f, "Invalid placeholder count")
            }
            CompilerError::NonStaticValue => {
                write!(f, "Encountered non-static value")
            }
            CompilerError::UndeclaredVariable(var) => {
                write!(f, "Undeclared variable: {var}")
            }
            CompilerError::ScopePopError => {
                write!(f, "Could not pop scope, stack is empty")
            }
            CompilerError::InvalidSlotName(name) => {
                write!(f, "Slot #{name} does not exist")
            }
            CompilerError::AssignmentToConst(name) => {
                write!(f, "Cannot assign to immutable variable: {name}")
            }
            CompilerError::OnceScopeUsedMultipleTimes => {
                write!(
                    f,
                    "Scope cannot be used multiple times, set 'once' to false to use a scope multiple times"
                )
            }
            CompilerError::AssignmentToImmutableValue(name) => {
                write!(f, "Cannot assign to immutable value: {name}")
            }
            CompilerError::AssignmentToImmutableReference(name) => {
                write!(f, "Cannot assign to immutable reference: {name}")
            }
            CompilerError::TypeError(err) => {
                write!(f, "Type error: {:#?}", err)
            }
            CompilerError::Spanned(err, span) => {
                write!(f, "{} at {:?}", err, span)
            }
            CompilerError::Multiple(errors) => {
                for err in errors {
                    writeln!(f, "{}", err)?;
                }
                Ok(())
            }
        }
    }
}
