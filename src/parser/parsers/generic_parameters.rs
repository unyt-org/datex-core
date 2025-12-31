use crate::ast::structs::r#type::TypeExpression;
use crate::parser::errors::ParserError;
use crate::parser::lexer::Token;
use crate::parser::{Parser, SpannedParserError};
use core::ops::Range;

impl Parser {
    pub(crate) fn parse_generic_parameters(
        &mut self,
    ) -> Result<(Vec<TypeExpression>, Range<usize>), SpannedParserError> {
        // expect <
        self.expect(Token::LeftAngle)?;

        let mut generic_params = Vec::new();
        let end_span = loop {
            // parse type expression
            let type_expr = self.parse_type_expression(0)?;
            generic_params.push(type_expr);

            // check for comma or >
            match self.peek()?.token {
                Token::Comma => {
                    self.advance()?;
                }
                Token::RightAngle => {
                    break self.advance()?.span;
                }
                _ => {
                    return Err(SpannedParserError {
                        error: crate::parser::errors::ParserError::UnexpectedToken {
                            expected: vec![Token::Comma, Token::RightAngle],
                            found: self.peek()?.token.clone(),
                        },
                        span: self.peek()?.span.clone(),
                    });
                }
            }
        };

        Ok((generic_params, end_span))
    }

    /// Tries to parse generic parameters enclosed in `<` and `>`.
    /// If the next tokens do not form valid generic parameters, it rolls back to the original position and returns an Err.
    pub(crate) fn try_parse_generic_parameters_or_roll_back(
        &mut self,
    ) -> Result<(Vec<TypeExpression>, Range<usize>), SpannedParserError> {
        let start_pos = self.pos;
        match self.try_parse_generic_parameters() {
            Ok(result) => Ok(result),
            Err(err) => {
                // roll back to the start position
                self.pos = start_pos;
                Err(err)
            }
        }
    }

    /// Tries to parse generic parameters enclosed in `<` and `>`.
    /// If the next tokens do not form valid generic parameters, it returns an Err.
    /// The caller should handle the rollback if needed.
    fn try_parse_generic_parameters(
        &mut self,
    ) -> Result<(Vec<TypeExpression>, Range<usize>), SpannedParserError> {
        // expect <
        self.expect(Token::LeftAngle)?;

        let mut generic_params = Vec::new();
        let end_span = loop {
            // parse type expression
            let type_expr = self.parse_type_expression(0)?;
            generic_params.push(type_expr);

            // check for comma or >
            match self.peek()?.token {
                Token::Comma => {
                    self.advance()?;
                }
                Token::RightAngle => {
                    break self.advance()?.span;
                }
                _ => {
                    // indicate that parsing generic parameters failed and should be rolled back
                    return Err(SpannedParserError {
                        error: ParserError::CouldNotMatchGenericParams,
                        span: 0..0,
                    });
                }
            }
        };

        Ok((generic_params, end_span))
    }
}
