use itertools::Itertools;
use crate::ast::structs::expression::DatexExpression;
use crate::ast::lexer::{SpannedToken, Token};
use crate::ast::structs::expression::{DatexExpressionData, List};
use crate::compiler::error::{collect_or_pass_error, ErrorCollector, MaybeAction};
use crate::parser::errors::{DetailedParserErrorsWithAst, ParserError, SpannedParserError};
// TODO: move to different module

mod errors;
mod parsers;

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    // when Some, collect all errors instead of returning on first error
    collected_errors: Option<Vec<SpannedParserError>>
}

impl Parser {

    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            pos: 0,
            collected_errors: None,
        }
    }

    /// Parses the tokens and collects all errors, returning them along with the final (possibly partial) AST.
    pub fn parse_and_collect_errors(&mut self) -> Result<DatexExpression, DetailedParserErrorsWithAst> {
        self.collected_errors = Some(Vec::new());
        match self.parse() {
            Err(_) => {
                unreachable!()
            }
            Ok(ast) => {
                if let Some(errors) = self.collected_errors.take() {
                    if errors.is_empty() {
                        Ok(ast)
                    } else {
                        Err(DetailedParserErrorsWithAst {
                            ast,
                            errors,
                        })
                    }
                } else {
                    Ok(ast)
                }
            }
        }
    }

    /// Parses the tokens and returns on the first error encountered.
    /// If no errors are found, returns the final, complete AST.
    pub fn parse_and_return_on_first_error(&mut self) -> Result<DatexExpression, SpannedParserError> {
        self.collected_errors = None;
        self.parse()
    }

    fn parse(&mut self) -> Result<DatexExpression, SpannedParserError> {
        println!("PARSING TOKENS:\n{}", self.tokens.iter().map(|t| &t.token).join("\n"));
        self.parse_expression(0)
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
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        Ok(tok)
    }

    fn expect(&mut self, token: Token) -> Result<SpannedToken, SpannedParserError> {
        let next_token = self.advance()?;
        if next_token.token != token {
            self.collect_error(SpannedParserError {
                error: ParserError::ExpectedToken(token),
                span: self.peek()?.span.clone(),
            })?;
        }
        Ok(next_token)
    }

    fn get_current_source_position(&self) -> usize {
        if self.pos == 0 {
            0
        } else if self.pos - 1 < self.tokens.len() {
            self.tokens[self.pos - 1].span.end
        } else if let Some(last) = self.tokens.last() {
            last.span.end
        } else {
            0
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::ast::lexer::get_spanned_tokens_from_source;
    use crate::ast::spanned::Spanned;
    use super::*;

    pub fn try_parse_and_return_on_first_error(src: &str) -> Result<DatexExpression, SpannedParserError> {
        let tokens = get_spanned_tokens_from_source(src).unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_and_return_on_first_error()
    }

    pub fn try_parse_and_collect_errors(src: &str) -> Result<DatexExpression, DetailedParserErrorsWithAst> {
        let tokens = get_spanned_tokens_from_source(src).unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_and_collect_errors()
    }

    pub fn parse(src: &str) -> DatexExpression {
        let tokens = get_spanned_tokens_from_source(src).unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_and_return_on_first_error().unwrap()
    }
}
