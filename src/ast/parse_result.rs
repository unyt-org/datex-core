use crate::ast::DatexExpression;
use crate::ast::error::error::ParseError;
use core::ops::Range;

#[derive(Debug, Clone)]
pub struct ValidDatexParseResult {
    pub ast: DatexExpression,
    pub spans: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
pub struct InvalidDatexParseResult {
    pub ast: Option<DatexExpression>,
    pub errors: Vec<ParseError>,
    pub spans: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
pub enum DatexParseResult {
    Valid(ValidDatexParseResult),
    Invalid(InvalidDatexParseResult),
}

impl DatexParseResult {
    pub fn is_valid(&self) -> bool {
        core::matches!(self, DatexParseResult::Valid { .. })
    }
    pub fn unwrap(self) -> ValidDatexParseResult {
        match self {
            DatexParseResult::Valid(result) => result,
            DatexParseResult::Invalid(InvalidDatexParseResult {
                errors,
                ..
            }) => {
                core::panic!("Parsing failed with errors: {:?}", errors)
            }
        }
    }
    pub fn errors(&self) -> &Vec<ParseError> {
        match self {
            DatexParseResult::Valid { .. } => {
                core::panic!("No errors in valid parse result")
            }
            DatexParseResult::Invalid(InvalidDatexParseResult {
                errors,
                ..
            }) => errors,
        }
    }
    pub fn spans(&self) -> &Vec<Range<usize>> {
        match self {
            DatexParseResult::Valid(ValidDatexParseResult {
                spans, ..
            }) => spans,
            DatexParseResult::Invalid(InvalidDatexParseResult {
                spans,
                ..
            }) => spans,
        }
    }

    pub fn to_result(self) -> Result<ValidDatexParseResult, Vec<ParseError>> {
        match self {
            DatexParseResult::Valid(result) => Ok(result),
            DatexParseResult::Invalid(InvalidDatexParseResult {
                errors,
                ..
            }) => Err(errors),
        }
    }
}
