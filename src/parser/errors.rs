use crate::ast::structs::expression::DatexExpression;
use crate::compiler::error::ErrorCollector;
use crate::global::operators::UnaryOperator;
use crate::parser::lexer::Token;
use crate::values::core_values::endpoint::InvalidEndpointError;
use crate::values::core_values::error::NumberParseError;
use std::ops::Range;

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
}

#[derive(Debug)]
pub struct DetailedParserErrorsWithAst {
    pub ast: DatexExpression, // TODO: rename to DatexAstNode
    pub errors: Vec<SpannedParserError>,
}

#[derive(Debug, Clone)]
pub struct SpannedParserError {
    pub error: ParserError,
    pub span: Range<usize>,
}

impl ErrorCollector<SpannedParserError> for Vec<SpannedParserError> {
    fn record_error(&mut self, error: SpannedParserError) {
        self.push(error);
    }
}
