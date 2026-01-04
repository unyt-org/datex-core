use crate::ast::expressions::{DatexExpression, DatexExpressionData, Map};
use crate::ast::spanned::Spanned;
use crate::parser::errors::ParserError;
use crate::parser::lexer::Token;
use crate::parser::{Parser, SpannedParserError};

impl Parser {
    pub(crate) fn parse_key(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {
            // allow integers as keys
            Token::IntegerLiteral(value) => {
                self.parse_integer_literal(value)?
            }
            // allow string literals as keys
            Token::StringLiteral(value) => self.parse_string_literal(value)?,

            // allow parenthesized statements as keys
            Token::LeftParen => self.parse_parenthesized_statements()?,

            // treat plain identifiers as text keys
            Token::Identifier(name) => {
                DatexExpressionData::Text(name).with_span(self.advance()?.span)
            }

            // map reserved keywords to text keys
            // TODO: add more keywords as needed
            t @ Token::True |
            t @ Token::False |
            t @ Token::TypeDeclaration |
            t @ Token::If |
            t @ Token::Else |
            t @ Token::Is |
            t @ Token::Matches |
            t @ Token::And |
            t @ Token::Or => {
                DatexExpressionData::Text(t.as_const_str().unwrap().to_string())
                    .with_span(self.advance()?.span)
            }

            _ => {
                return Err(SpannedParserError {
                    error: ParserError::UnexpectedToken {
                        expected: vec![
                            Token::Identifier("".to_string()),
                            Token::IntegerLiteral("".to_string()),
                            Token::StringLiteral("".to_string()),
                            Token::LeftParen,
                        ],
                        found: self.peek()?.token.clone(),
                    },
                    span: self.peek()?.span.clone(),
                });
            }
        })
    }
}
