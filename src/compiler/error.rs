use crate::ast::error::error::{ParseError, SpanOrToken};
use crate::ast::tree::DatexExpression;
use std::fmt::{Display, Formatter};
use std::ops::Range;
use chumsky::prelude::SimpleSpan;
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
    ParseError(ParseError),
}

/// A compiler error that can be linked to a specific span in the source code
#[derive(Debug)]
pub struct SpannedCompilerError {
    pub error: CompilerError,
    pub span: Option<Range<usize>>
}

impl SpannedCompilerError {
    pub fn new_with_simple_span(error: CompilerError, span: SimpleSpan) -> SpannedCompilerError {
        SpannedCompilerError {
            error,
            span: Some(span.start..span.end)
        }
    }
}

impl Display for SpannedCompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.span.as_ref().map(|s|format!("{}..{}", s.start, s.end)).unwrap_or("?".to_string()))
    }
}

impl From<ParseError> for SpannedCompilerError {
    fn from(value: ParseError) -> Self {
        SpannedCompilerError {
            span: match &value.span {
                SpanOrToken::Span(range) => Some(range.clone()),
                _ => panic!("expected byte range, got token span")
            },
            error: CompilerError::ParseError(value),
        }
    }
}

impl From<CompilerError> for SpannedCompilerError {
    fn from(value: CompilerError) -> Self {
        SpannedCompilerError {
            error: value,
            span: None
        }
    }
}

#[derive(Debug, Default)]
pub struct DetailedCompilerError {
    pub errors: Vec<SpannedCompilerError>
}

impl Display for DetailedCompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for error in self.errors.iter() {
            write!(f, "{}\n", error)?;
        }
        Ok(())
    }
}

impl DetailedCompilerError {
    pub fn record_error_with_span(&mut self, error: CompilerError, span: Range<usize>) {
        self.errors.push(SpannedCompilerError { error, span: Some(span) });
    }
    pub fn record_error(&mut self, error: SpannedCompilerError) {
        self.errors.push(error);
    }
}

#[derive(Debug)]
pub enum DetailedOrSimpleCompilerError {
    Detailed(DetailedCompilerError),
    Simple(SpannedCompilerError),
}

impl From<CompilerError> for DetailedOrSimpleCompilerError {
    fn from(value: CompilerError) -> Self {
        DetailedOrSimpleCompilerError::Simple(SpannedCompilerError::from(value))
    }
}

impl From<SpannedCompilerError> for DetailedOrSimpleCompilerError {
    fn from(value: SpannedCompilerError) -> Self {
        DetailedOrSimpleCompilerError::Simple(value)
    }
}


impl From<Vec<ParseError>> for DetailedCompilerError {
    fn from(value: Vec<ParseError>) -> Self {
        DetailedCompilerError {
            errors: value
                .into_iter()
                .map(SpannedCompilerError::from)
                .collect()
        }
    }
}

impl From<TypeError> for DetailedOrSimpleCompilerError {
    fn from(value: TypeError) -> Self {
        // TODO: also store and map span from type error
        DetailedOrSimpleCompilerError::Simple(SpannedCompilerError::from(CompilerError::from(value)))
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
            CompilerError::ParseError(err) => {
                write!(f, "{:?}", err)
            }
        }
    }
}
