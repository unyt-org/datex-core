use std::str::FromStr;
use crate::values::core_values::decimal::Decimal;
use crate::parser::lexer::{IntegerLiteral, DecimalLiteral, Token};
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Slot};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;
use crate::parser::utils::unescape_text;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::error::NumberParseError;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::{IntegerTypeVariant, TypedInteger};
use crate::values::pointer::PointerAddress;

impl Parser {
    pub(crate) fn parse_atom(&mut self) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            Token::LeftCurly => self.parse_map()?,
            Token::LeftBracket => self.parse_list()?,
            Token::LeftParen => self.parse_parenthesized_statements()?,

            Token::Placeholder => {
                DatexExpressionData::Placeholder
                    .with_span(self.advance()?.span)
            }

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
            Token::NamedSlot(slot_name) => {
                DatexExpressionData::Slot(Slot::Named(slot_name[1..].to_string()))
                    .with_span(self.advance()?.span)
            }
            Token::Slot(slot_address) => {
                DatexExpressionData::Slot(Slot::Addressed(slot_address[1..].parse::<u32>().unwrap()))
                    .with_span(self.advance()?.span)
            }
            Token::PointerAddress(address) => {
                DatexExpressionData::PointerAddress(PointerAddress::try_from(&address[1..]).unwrap())
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

            Token::IntegerLiteral(IntegerLiteral { value, variant }) => {
                let span = self.advance()?.span.clone();
                let res = match variant {
                    Some(var) => TypedInteger::from_string_radix_with_variant(&value, 10, var)
                        .map(DatexExpressionData::TypedInteger),
                    None => Integer::from_string_radix(&value, 10)
                        .map(DatexExpressionData::Integer),
                };
                match res {
                    Ok(expr) => expr.with_span(span),
                    Err(e) => return self.collect_error_and_continue(SpannedParserError {
                        error: ParserError::NumberParseError(e),
                        span,
                    }),
                }
            },
            Token::BinaryIntegerLiteral(IntegerLiteral { value, variant })
            | Token::HexadecimalIntegerLiteral(IntegerLiteral { value, variant })
            | Token::OctalIntegerLiteral(IntegerLiteral { value, variant }) => {
                let token = self.advance()?;
                let radix = match token.token {
                    Token::BinaryIntegerLiteral(_) => 2,
                    Token::OctalIntegerLiteral(_) => 8,
                    Token::HexadecimalIntegerLiteral(_) => 16,
                    _ => unreachable!(),
                };
                let res = match variant {
                    Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], radix, var)
                        .map(DatexExpressionData::TypedInteger),
                    None => Integer::from_string_radix(&value[2..], radix)
                        .map(DatexExpressionData::Integer),
                };
                match res {
                    Ok(expr) => expr.with_span(token.span),
                    Err(e) => return self.collect_error_and_continue(SpannedParserError {
                        error: ParserError::NumberParseError(e),
                        span: token.span,
                    }),
                }
            },

            Token::DecimalLiteral(DecimalLiteral { value, variant }) => {
                let span = self.advance()?.span.clone();
                let res = match variant {
                    Some(var) => TypedDecimal::from_string_and_variant_in_range(&value, var)
                        .map(DatexExpressionData::TypedDecimal),
                    None => Decimal::from_string(&value)
                        .map(DatexExpressionData::Decimal),
                };
                match res {
                    Ok(expr) => expr.with_span(span),
                    Err(e) => return self.collect_error_and_continue(SpannedParserError {
                        error: ParserError::NumberParseError(e),
                        span,
                    }),
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
                        Token::Identifier("".to_string()),
                        Token::StringLiteral("".to_string()),
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
    use std::assert_matches::assert_matches;
    use crate::parser::tests::try_parse_and_return_on_first_error;
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{DatexExpressionData, Slot, Statements};
    use crate::parser::errors::ParserError;
    use crate::parser::parser_result::ParserResult;
    use crate::parser::tests::{parse, try_parse_and_collect_errors};
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::endpoint::{Endpoint, InvalidEndpointError};
    use crate::values::core_values::integer::typed_integer::TypedInteger;
    use crate::values::pointer::PointerAddress;

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
        assert_matches!(result, ParserResult::Invalid { .. });
        let ast = result.ast();
        let errors = result.errors().unwrap();

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

    #[test]
    fn parse_named_slot() {
        let expr = parse("#mySlot");
        assert_eq!(expr.data, DatexExpressionData::Slot(Slot::Named("mySlot".to_string())));
    }

    #[test]
    fn parse_addressed_slot() {
        let expr = parse("#42");
        assert_eq!(expr.data, DatexExpressionData::Slot(Slot::Addressed(42)));
    }

    #[test]
    fn parse_pointer_address() {
        let expr = parse("$ABCDEF");
        assert_eq!(expr.data, DatexExpressionData::PointerAddress(PointerAddress::try_from("ABCDEF").unwrap()));
    }

    #[test]
    fn parse_placeholder() {
        let expr = parse("?");
        assert_eq!(expr.data, DatexExpressionData::Placeholder);
    }

    #[test]
    fn parse_integer_literal() {
        let expr = parse("12345");
        assert_eq!(expr.data, DatexExpressionData::Integer(12345.into()));
    }

    #[test]
    fn parse_integer_literal_with_underscores() {
        let expr = parse("12_345_678");
        assert_eq!(expr.data, DatexExpressionData::Integer(12345678.into()));
    }

    #[test]
    fn parse_negative_integer_literal() {
        let expr = parse("-6789");
        assert_eq!(expr.data, DatexExpressionData::Integer((-6789).into()));
    }

    #[test]
    fn parse_typed_integer_literal() {
        let expr = parse("12345i32");
        assert_eq!(expr.data, DatexExpressionData::TypedInteger(
            TypedInteger::I32(12345)
        ));
    }

    #[test]
    fn parse_hex_integer_literal() {
        let expr = parse("0x1A3F");
        assert_eq!(expr.data, DatexExpressionData::Integer(0x1A3F.into()));
    }

    #[test]
    fn parse_binary_integer_literal() {
        let expr = parse("0b1101");
        assert_eq!(expr.data, DatexExpressionData::Integer(0b1101.into()));
    }

    #[test]
    fn parse_octal_integer_literal() {
        let expr = parse("0o755");
        assert_eq!(expr.data, DatexExpressionData::Integer(0o755.into()));
    }

    #[test]
    fn parse_typed_hex_integer_literal() {
        let expr = parse("0xFFu8");
        assert_eq!(expr.data, DatexExpressionData::TypedInteger(
            TypedInteger::U8(0xFF)
        ));
    }

    #[test]
    fn parse_typed_binary_integer_literal() {
        let expr = parse("0b1010i16");
        assert_eq!(expr.data, DatexExpressionData::TypedInteger(
            TypedInteger::I16(0b1010)
        ));
    }

    #[test]
    fn parse_typed_octal_integer_literal() {
        let expr = parse("0o77u32");
        assert_eq!(expr.data, DatexExpressionData::TypedInteger(
            TypedInteger::U32(0o77)
        ));
    }

    #[test]
    fn parse_decimal_literal() {
        let expr = parse("123.456");
        assert_eq!(expr.data, DatexExpressionData::Decimal(Decimal::from_string("123.456").unwrap()));
    }

    #[test]
    fn parse_typed_decimal_literal() {
        let expr = parse("78.9f32");
        assert_eq!(expr.data, DatexExpressionData::TypedDecimal(
            TypedDecimal::F32(78.9.into())
        ));
    }

    #[test]
    fn parse_negative_decimal_literal() {
        let expr = parse("-0.001");
        assert_eq!(expr.data, DatexExpressionData::Decimal(Decimal::from_string("-0.001").unwrap()));
    }
}