use crate::ast::spanned::Spanned;
use crate::parser::lexer::Token;
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;

impl Parser {
    pub(crate) fn parse_type_key(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            // allow grouped expressions as keys
            Token::LeftParen => self.parse_type_grouped()?,
            
            // allow integers as keys
            Token::IntegerLiteral(value) => self.parse_type_integer_literal(value)?,
            // allow string literals as keys
            Token::StringLiteral(value) => self.parse_type_string_literal(value)?,

            // treat plain identifiers as text keys
            Token::Identifier(name) => {
                TypeExpressionData::Text(name)
                    .with_span(self.advance()?.span)
            }
            // map reserved keywords to text keys
            Token::True => {
                TypeExpressionData::Text("true".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::False => {
                TypeExpressionData::Text("false".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::TypeDeclaration => {
                TypeExpressionData::Text("type".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::If => {
                TypeExpressionData::Text("if".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::Else => {
                TypeExpressionData::Text("else".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::Is => {
                TypeExpressionData::Text("is".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::And => {
                TypeExpressionData::Text("and".to_string())
                    .with_span(self.advance()?.span)
            }
            Token::Or => {
                TypeExpressionData::Text("or".to_string())
                    .with_span(self.advance()?.span)
            }
            // TODO: add more keywords as needed


            _ => return Err(SpannedParserError {
                error: ParserError::UnexpectedToken {
                    expected: vec![
                        Token::Identifier("".to_string()),
                        Token::IntegerLiteral("".to_string()),
                        Token::StringLiteral("".to_string()),
                    ],
                    found: self.peek()?.token.clone(),
                },
                span: self.peek()?.span.clone()
            }),

        })
    }
}