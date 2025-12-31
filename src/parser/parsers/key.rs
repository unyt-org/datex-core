use crate::ast::spanned::Spanned;
use crate::parser::lexer::Token;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Map};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;

impl Parser {
    pub(crate) fn parse_key(&mut self) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            // allow integers as keys
            Token::IntegerLiteral(value) => self.parse_integer_literal(value)?,
            // allow string literals as keys
            Token::StringLiteral(value) => self.parse_string_literal(value)?,

            // allow parenthesized statements as keys
            Token::LeftParen => self.parse_parenthesized_statements()?,

            // treat plain identifiers as text keys
            Token::Identifier(name) => {
                DatexExpressionData::Text(name)
                    .with_span(self.advance()?.span)
            }
            // map reserved keywords to text keys
            Token::True => {
                DatexExpressionData::Text("true".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::False => {
                DatexExpressionData::Text("false".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::TypeDeclaration => {
                DatexExpressionData::Text("type".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::If => {
                DatexExpressionData::Text("if".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::Else => {
                DatexExpressionData::Text("else".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::Is => {
                DatexExpressionData::Text("is".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::And => {
                DatexExpressionData::Text("and".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::Or => {
                DatexExpressionData::Text("or".to_string())
                    .with_span(self.advance()?.span)
            }
            // TODO: add more keywords as needed


            _ => return Err(SpannedParserError {
                error: ParserError::UnexpectedToken {
                    expected: vec![
                        Token::Identifier("".to_string()),
                        Token::IntegerLiteral("".to_string()),
                        Token::StringLiteral("".to_string()),
                        Token::LeftParen,
                    ],
                    found: self.peek()?.token.clone(),
                },
                span: self.peek()?.span.clone()
            }),

        })
    }
}