use std::str::FromStr;
use crate::values::core_values::decimal::Decimal;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;
use crate::parser::utils::unescape_text;
use crate::values::core_values::endpoint::Endpoint;

impl Parser {
    pub(crate) fn parse_atom(&mut self) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            Token::LeftCurly => self.parse_map()?,
            Token::LeftBracket => self.parse_list()?,
            Token::LeftParen => self.parse_parenthesized_statements()?,

            Token::True => {
                DatexExpressionData::Boolean(true)
                    .with_span(self.advance()?.span)
            }
            Token::False => {
                DatexExpressionData::Boolean(false)
                    .with_span(self.advance()?.span)
            }
            Token::Null => {
                DatexExpressionData::Null
                    .with_span(self.advance()?.span)
            }
            Token::Identifier(name) => {
                DatexExpressionData::Identifier(name)
                    .with_span(self.advance()?.span)
            }
            Token::StringLiteral(value) => {
                DatexExpressionData::Text(unescape_text(&value))
                    .with_span(self.advance()?.span)
            }
            Token::Infinity => {
                DatexExpressionData::Decimal(Decimal::Infinity)
                    .with_span(self.advance()?.span)
            }
            Token::Nan => {
                DatexExpressionData::Decimal(Decimal::Nan)
                    .with_span(self.advance()?.span)
            }
            Token::Endpoint(endpoint_name) => {
                let span = self.advance()?.span.clone();
                match Endpoint::from_str(endpoint_name.as_str()) {
                    Err(e) => return self
                        .collect_error_and_continue(SpannedParserError {
                            error: ParserError::InvalidEndpointName {name: endpoint_name, details: e},
                            span,
                        }),
                    Ok(endpoint) => {
                        DatexExpressionData::Endpoint(endpoint).with_span(span)
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
    use datex_core::parser::tests::try_parse_and_return_on_first_error;
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{DatexExpressionData, Statements};
    use crate::parser::errors::ParserError;
    use crate::parser::tests::{parse, try_parse_and_collect_errors};
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::endpoint::{Endpoint, InvalidEndpointError};

    #[test]
    fn parse_boolean_true() {
        let expr = parse("true");
        assert_eq!(expr.data, DatexExpressionData::Boolean(true));
    }

    #[test]
    fn parse_boolean_false() {
        let expr = parse("false");
        assert_eq!(expr.data, DatexExpressionData::Boolean(false));
    }

    #[test]
    fn parse_null() {
        let expr = parse("null");
        assert_eq!(expr.data, DatexExpressionData::Null);
    }

    #[test]
    fn parse_identifier() {
        let expr = parse("myVar");
        assert_eq!(expr.data, DatexExpressionData::Identifier("myVar".to_string()));
    }

    #[test]
    fn parse_string_literal() {
        let expr = parse("\"Hello, World!\"");
        assert_eq!(expr.data, DatexExpressionData::Text("Hello, World!".to_string()));

        let expr2 = parse("'Single quotes'");
        assert_eq!(expr2.data, DatexExpressionData::Text("Single quotes".to_string()));
    }

    #[test]
    fn parse_infinity() {
        let expr = parse("infinity");
        assert_eq!(expr.data, DatexExpressionData::Decimal(Decimal::Infinity));
    }

    #[test]
    fn parse_nan() {
        let expr = parse("nan");
        assert_eq!(expr.data, DatexExpressionData::Decimal(Decimal::Nan));
    }

    #[test]
    fn parse_endpoint() {
        let expr = parse("@example");
        assert_eq!(expr.data, DatexExpressionData::Endpoint(Endpoint::new("@example")));
    }

    #[test]
    fn parse_invalid_endpoint() {
        let result = try_parse_and_return_on_first_error("@x");
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().error,
            ParserError::InvalidEndpointName {
                name: "@x".to_string(),
                details: InvalidEndpointError::MinLengthNotMet
            }
        );
    }
    
    #[test]
    fn parse_invalid_endpoint_and_continue() {
        let result = try_parse_and_collect_errors("@x; true");
        assert!(result.is_err());
        let result = result.unwrap_err();
        let ast = result.ast;
        let errors = result.errors;

        assert_eq!(ast.data, DatexExpressionData::Statements(Statements {
            statements: vec![
                DatexExpressionData::Recover.with_default_span(),
                DatexExpressionData::Boolean(true).with_default_span(),
            ],
            is_terminated: false,
            unbounded: None,
        }));
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].error,
            ParserError::InvalidEndpointName {
                name: "@x".to_string(),
                details: InvalidEndpointError::MinLengthNotMet
            }
        );
    }
}