use std::ops::Range;
use crate::ast::lexer::Token;
use crate::ast::structs::expression::DatexExpression;
use crate::compiler::error::ErrorCollector;
use crate::values::core_values::endpoint::InvalidEndpointError;
use crate::values::core_values::error::NumberParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    UnexpectedToken {
        expected: Vec<Token>,
        found: Token,
    },
    ExpectedMoreTokens,
    InvalidEndpointName {
        name: String,
        details: InvalidEndpointError
    },
    InvalidAssignmentTarget,
    NumberParseError(NumberParseError)
}

#[derive(Debug)]
pub struct DetailedParserErrorsWithAst {
    pub ast: DatexExpression, // TODO: rename to DatexAstNode
    pub errors: Vec<SpannedParserError>,
}

#[derive(Debug)]
pub struct SpannedParserError {
    pub error: ParserError,
    pub span: Range<usize>,
}

impl ErrorCollector<SpannedParserError> for Vec<SpannedParserError> {
    fn record_error(&mut self, error: SpannedParserError) {
        self.push(error);
    }
}