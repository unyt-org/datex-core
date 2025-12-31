use core::ops::Range;
use itertools::Itertools;
use parser_result::ParserResult;
use crate::ast::structs::expression::DatexExpression;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::DatexExpressionData;
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::compiler::error::{collect_or_pass_error, ErrorCollector, MaybeAction};
use crate::parser::errors::{ParserError, SpannedParserError};
use crate::parser::lexer::{SpannedToken, Token};
use crate::parser::parser_result::{InvalidDatexParseResult, ValidDatexParseResult};
// TODO: move to different module

pub mod errors;
mod parsers;
pub mod utils;
pub mod lexer;
pub mod parser_result;


pub enum ParseResult<T> {
    RecoveredFromError,
    Ok(T)
}


pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    // when Some, collect all errors instead of returning on first error
    collected_errors: Option<Vec<SpannedParserError>>,
}

impl Parser {

    /// Parses the given source code.
    /// Collects all lexing and parsing errors encountered.
    pub fn parse_collecting(src: &str) -> ParserResult {
        let (tokens, errors) = lexer::get_spanned_tokens_from_source(src);
        let mut parser = Self::new_from_tokens(tokens, Some(errors));
        match parser.parse_root() {
            // this should never happen when collecting errors
            Err(e) => {
                unreachable!("An error was not correctly handled during parsing: {:#?}", e);
            }
            Ok(ast) => {
                // has errors, return invalid result
                if let Some(errors) = parser.collected_errors && !errors.is_empty() {
                    ParserResult::Invalid(InvalidDatexParseResult {
                        ast,
                        errors,
                    })
                }
                // has no errors, return valid result
                else {
                    ParserResult::Valid(ValidDatexParseResult { ast })
                }
            }
        }

    }

    /// Parses the given source code.
    /// Aborts on the first lexing or parsing error encountered.
    pub fn parse(src: &str) -> Result<DatexExpression, SpannedParserError> {
        let (tokens, errors) = lexer::get_spanned_tokens_from_source(src);
        // already has lexer errors - aborts early when parsing starts
        if let Some(first_error) = errors.into_iter().next() {
            Err(first_error)
        }
        // no lexer errors - can proceed with parsing (using early abort mode)
        else {
            let mut parser = Self::new_from_tokens(tokens, None);
            parser.parse_root()
        }
    }

    fn new_from_tokens(tokens: Vec<SpannedToken>, collected_errors: Option<Vec<SpannedParserError>>) -> Self {
        Self {
            tokens,
            pos: 0,
            collected_errors,
        }
    }

    /// Entrypoint for parsing a full source file.
    fn parse_root(&mut self) -> Result<DatexExpression, SpannedParserError> {
        self.parse_top_level_statements()
    }


    /// Collects an error if detailed error collection is enabled,
    /// or returns the error as Err()
    fn collect_error(
        &mut self,
        error: SpannedParserError,
    ) -> Result<(), SpannedParserError> {
        match &mut self.collected_errors {
            Some(collected_errors) => {
                collected_errors.record_error(error);
                Ok(())
            }
            None => Err(error),
        }
    }

    /// Collects an error and returns a Recover expression to continue parsing if
    /// detailed error collection is enabled,
    /// or returns the error as Err()
    fn collect_error_and_continue(
        &mut self,
        error: SpannedParserError,
    ) -> Result<DatexExpression, SpannedParserError> {
        let span = error.span.clone();
        self.collect_error(error).map(|_| {
            DatexExpressionData::Recover.with_span(span)
        })
    }

    /// Collects an error and returns a Recover type expression to continue parsing if
    /// detailed error collection is enabled,
    /// or returns the error as Err()
    fn collect_error_and_continue_with_type_expression(
        &mut self,
        error: SpannedParserError,
    ) -> Result<TypeExpression, SpannedParserError> {
        let span = error.span.clone();
        self.collect_error(error).map(|_| {
            TypeExpressionData::Recover.with_span(span)
        })
    }

    /// Collects the Err variant of the Result if detailed error collection is enabled,
    /// or returns the Result mapped to a MaybeAction.
    fn collect_result<T>(
        &mut self,
        result: Result<T, SpannedParserError>,
    ) -> Result<MaybeAction<T>, SpannedParserError> {
        collect_or_pass_error(&mut self.collected_errors, result)
    }


    fn peek(&self) -> Result<&SpannedToken, SpannedParserError> {
        if self.pos >= self.tokens.len() {
            Err(SpannedParserError {
                error: ParserError::ExpectedMoreTokens,
                span: if let Some(last) = self.tokens.last() {
                    last.span.end..last.span.end
                } else {
                    0..0
                },
            })
        } else {
            Ok(&self.tokens[self.pos])
        }
    }
    

    fn has_more_tokens(&self) -> bool {
        self.pos < self.tokens.len()
    }

    fn advance(&mut self) -> Result<SpannedToken, SpannedParserError> {
        if self.pos >= self.tokens.len() {
            return Err(SpannedParserError {
                error: ParserError::ExpectedMoreTokens,
                span: if let Some(last) = self.tokens.last() {
                    last.span.end..last.span.end
                } else {
                    0..0
                },
            });
        }
        let tok = self.tokens[self.pos].clone(); // TODO: take, don't clone?
        self.pos += 1;
        Ok(tok)
    }

    fn expect(&mut self, token: Token) -> Result<SpannedToken, SpannedParserError> {
        let next_token = self.advance()?;
        if next_token.token != token {
            self.collect_error(SpannedParserError {
                error: ParserError::UnexpectedToken {
                    expected: vec![token],
                    found: next_token.token.clone(),
                },
                span: self.peek()?.span.clone(),
            })?;
        }
        Ok(next_token)
    }

    fn expect_identifier(&mut self) -> Result<(String, Range<usize>), SpannedParserError> {
        match self.advance()? {
            SpannedToken { token: Token::Identifier(identifier), span }  => Ok((identifier, span)),
            token => {
                Err(SpannedParserError {
                    error: ParserError::UnexpectedToken {
                        expected: vec![Token::Identifier("identifier".to_string())],
                        found: token.token.clone(),
                    },
                    span: token.span.clone()
                })
            }
        }
    }

    fn get_current_source_position(&self) -> usize {
        if let Some(token) = self.tokens.get(self.pos) {
            token.span.start
        } else if let Some(last_token) = self.tokens.last() {
            last_token.span.end
        } else {
            0
        }
    }

    /// Attempt to recover from a parsing error by skipping tokens until one of the recovery tokens is found.
    /// If recovery is successful after an error result was provided, returns `ParseResult::RecoveredFromError`.
    /// If the result was Ok, returns `RecoverState::Ok` with the parsed value
    /// If error collection is not enabled in the parser, the error is returned directly in the result and can be bubbled up.
    fn recover_on_error<T>(
        &mut self,
        result: Result<T, SpannedParserError>,
        recovery_tokens: &[Token],
    ) -> Result<ParseResult<T>, SpannedParserError> {
        match result {
            Ok(statement) => Ok(ParseResult::Ok(statement)),
            Err(err) => {
                self.collect_error(err)?;
                // attempt to recover by skipping to next semicolon or right paren
                while self.has_more_tokens() {
                    let token = &self.peek()?.token;
                    if recovery_tokens.contains(token) {
                        break;
                    }
                    self.advance()?;
                }
                Ok(ParseResult::RecoveredFromError)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    pub fn try_parse_and_return_on_first_error(src: &str) -> Result<DatexExpression, SpannedParserError> {
        Parser::parse(src)
    }

    pub fn try_parse_and_collect_errors(src: &str) -> ParserResult {
        Parser::parse_collecting(src)
    }

    pub fn parse(src: &str) -> DatexExpression {
        Parser::parse(src).unwrap()
    }
}
