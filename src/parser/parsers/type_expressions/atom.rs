use std::str::FromStr;
use crate::values::core_values::decimal::Decimal;
use crate::parser::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData};
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;
use crate::parser::utils::unescape_text;
use crate::values::core_values::endpoint::Endpoint;

impl Parser {
    pub(crate) fn parse_type_atom(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            Token::LeftCurly => self.parse_type_map()?,
            Token::LeftBracket => self.parse_type_list()?,
            Token::LeftParen => self.parse_type_grouped()?,

            Token::True => {
                TypeExpressionData::Boolean(true)
                    .with_span(self.advance()?.span)
            }
            Token::False => {
                TypeExpressionData::Boolean(false)
                    .with_span(self.advance()?.span)
            }
            Token::Null => {
                TypeExpressionData::Null
                    .with_span(self.advance()?.span)
            }
            Token::Identifier(name) => {
                TypeExpressionData::Identifier(name)
                    .with_span(self.advance()?.span)
            }
            Token::StringLiteral(value) => {
                TypeExpressionData::Text(unescape_text(&value))
                    .with_span(self.advance()?.span)
            }
            Token::Infinity => {
                TypeExpressionData::Decimal(Decimal::Infinity)
                    .with_span(self.advance()?.span)
            }
            Token::Nan => {
                TypeExpressionData::Decimal(Decimal::Nan)
                    .with_span(self.advance()?.span)
            }
            Token::Endpoint(endpoint_name) => {
                let span = self.advance()?.span.clone();
                match Endpoint::from_str(endpoint_name.as_str()) {
                    Err(e) => return self
                        .collect_error_and_continue_with_type_expression(SpannedParserError {
                            error: ParserError::InvalidEndpointName {name: endpoint_name, details: e},
                            span,
                        }),
                    Ok(endpoint) => {
                        TypeExpressionData::Endpoint(endpoint).with_span(span)
                    }
                }
            }

            _ => return Err(SpannedParserError {
                error: ParserError::UnexpectedToken {
                    expected: vec![
                        Token::LeftCurly,
                        Token::LeftBracket,
                        Token::LeftParen,
                        Token::True,
                        Token::False,
                        Token::Null,
                        Token::Identifier("<identifier>".to_string()),
                        Token::StringLiteral("<string>".to_string()),
                        Token::Infinity,
                        Token::Nan,
                    ],
                    found: self.peek()?.token.clone(),
                },
                span: self.peek()?.span.clone()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use datex_core::parser::parsers::type_expressions::tests::parse_type_expression;
    use crate::ast::structs::r#type::TypeExpressionData;
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::endpoint::{Endpoint};

    #[test]
    fn parse_boolean_true() {
        let expr = parse_type_expression("true");
        assert_eq!(expr.data, TypeExpressionData::Boolean(true));
    }

    #[test]
    fn parse_boolean_false() {
        let expr = parse_type_expression("false");
        assert_eq!(expr.data, TypeExpressionData::Boolean(false));
    }

    #[test]
    fn parse_null() {
        let expr = parse_type_expression("null");
        assert_eq!(expr.data, TypeExpressionData::Null);
    }

    #[test]
    fn parse_identifier() {
        let expr = parse_type_expression("myVar");
        assert_eq!(expr.data, TypeExpressionData::Identifier("myVar".to_string()));
    }

    #[test]
    fn parse_string_literal() {
        let expr = parse_type_expression("\"Hello, World!\"");
        assert_eq!(expr.data, TypeExpressionData::Text("Hello, World!".to_string()));

        let expr2 = parse_type_expression("'Single quotes'");
        assert_eq!(expr2.data, TypeExpressionData::Text("Single quotes".to_string()));
    }

    #[test]
    fn parse_infinity() {
        let expr = parse_type_expression("infinity");
        assert_eq!(expr.data, TypeExpressionData::Decimal(Decimal::Infinity));
    }

    #[test]
    fn parse_nan() {
        let expr = parse_type_expression("nan");
        assert_eq!(expr.data, TypeExpressionData::Decimal(Decimal::Nan));
    }

    #[test]
    fn parse_endpoint() {
        let expr = parse_type_expression("@example");
        assert_eq!(expr.data, TypeExpressionData::Endpoint(Endpoint::new("@example")));
    }

}