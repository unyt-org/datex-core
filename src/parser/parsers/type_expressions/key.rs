use crate::ast::expressions::DatexExpressionData;
use crate::ast::spanned::Spanned;
use crate::ast::type_expressions::{TypeExpression, TypeExpressionData};
use crate::parser::errors::ParserError;
use crate::parser::lexer::Token;
use crate::parser::{Parser, SpannedParserError};

impl Parser {
    pub(crate) fn parse_type_key(
        &mut self,
    ) -> Result<TypeExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {
            // allow grouped expressions as keys
            Token::LeftParen => self.parse_type_grouped()?,

            // allow integers as keys
            Token::IntegerLiteral(value) => {
                self.parse_type_integer_literal(value)?
            }
            // allow string literals as keys
            Token::StringLiteral(value) => {
                self.parse_type_string_literal(value)?
            }

            // treat plain identifiers as text keys
            Token::Identifier(name) => {
                TypeExpressionData::Text(name).with_span(self.advance()?.span)
            }
            // map reserved keywords to text keys
            // TODO: add more keywords as needed
            t @ Token::True
            | t @ Token::False
            | t @ Token::TypeDeclaration
            | t @ Token::If
            | t @ Token::Else
            | t @ Token::Is
            | t @ Token::Matches
            | t @ Token::And
            | t @ Token::Or => {
                TypeExpressionData::Text(t.as_const_str().unwrap().to_string())
                    .with_span(self.advance()?.span)
            }

            _ => {
                return Err(SpannedParserError {
                    error: ParserError::UnexpectedToken {
                        expected: vec![
                            Token::Identifier("".to_string()),
                            Token::IntegerLiteral("".to_string()),
                            Token::StringLiteral("".to_string()),
                        ],
                        found: self.peek()?.token.clone(),
                    },
                    span: self.peek()?.span.clone(),
                });
            }
        })
    }
}
