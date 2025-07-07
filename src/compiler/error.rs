use crate::compiler::ast_parser::{DatexExpression, ParserError};
use std::fmt::Display;
#[derive(Debug)]
pub enum CompilerError {
    UnexpectedTerm(DatexExpression),
    ParserErrors(Vec<ParserError>),
    SerializationError(binrw::Error),
    BigDecimalOutOfBoundsError,
    IntegerOutOfBoundsError,
    InvalidPlaceholderCount,
    NonStaticValue,
    UndeclaredVariable(String),
    ScopePopError,
}
impl From<Vec<ParserError>> for CompilerError {
    fn from(value: Vec<ParserError>) -> Self {
        CompilerError::ParserErrors(value)
    }
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::UnexpectedTerm(rule) => {
                write!(f, "Unexpected term: {rule:?}")
            }
            CompilerError::ParserErrors(error) => {
                write!(f, "Syntax error") // TODO
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
                write!(f, "Use of undeclared variable: {var}")
            }
            CompilerError::ScopePopError => {
                write!(f, "Could not pop scope, stack is empty")
            }
        }
    }
}
