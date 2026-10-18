use crate::{
    ast::expressions::DatexExpression,
    global::operators::UnaryOperator,
    parser::lexer::Token,
    values::core_values::{
        endpoint::InvalidEndpointError, error::NumberParseError,
    },
};

use crate::{prelude::*, utils::maybe_action::ErrorCollector};
use core::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    /// invalid token encountered during lexing
    InvalidToken,
    /// unexpected token encountered during parsing
    UnexpectedToken {
        expected: Vec<Token>,
        found: Token,
    },
    ExpectedMoreTokens,
    InvalidEndpointName {
        name: String,
        details: InvalidEndpointError,
    },
    InvalidAssignmentTarget,
    NumberParseError(NumberParseError),
    InvalidUnaryOperation {
        operator: UnaryOperator,
    },
    InvalidTypeVariantAccess,
    // used in internal parser logic to indicate a failed parse attempt that lead to a rollback
    CouldNotMatchGenericParams,

    ExpressionNestingTooDeep,
}

#[derive(Debug)]
pub struct DetailedParserErrorsWithAst {
    pub ast: DatexExpression, // TODO #657: rename to DatexAstNode
    pub errors: Vec<SpannedParserError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedParserError {
    pub error: ParserError,
    pub span: Range<usize>,
}

impl SpannedParserError {
    pub fn new(error: ParserError, span: Range<usize>) -> Self {
        Self { error, span }
    }
    pub fn with_span(mut self, span: Range<usize>) -> Self {
        self.span = span;
        self
    }

    /// If the error is an UnexpectedToken, update the expected tokens to the provided list.
    pub fn with_expected_tokens(mut self, expected: Vec<Token>) -> Self {
        if let ParserError::UnexpectedToken {
            found: _,
            expected: old,
        } = &mut self.error
        {
            for token in expected {
                if !old.contains(&token) {
                    old.push(token);
                }
            }
        }
        self
    }
}

impl ErrorCollector<SpannedParserError> for Vec<SpannedParserError> {
    fn record_error(&mut self, error: SpannedParserError) {
        self.push(error);
    }
}
