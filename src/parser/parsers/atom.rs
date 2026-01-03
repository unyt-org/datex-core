use crate::ast::expressions::{DatexExpression, DatexExpressionData, Slot};
use crate::ast::spanned::Spanned;
use crate::parser::errors::ParserError;
use crate::parser::lexer::{DecimalWithVariant, IntegerWithVariant, Token};
use crate::parser::utils::{
    IntegerOrDecimal, IntegerOrTypedInteger, parse_integer_literal,
    parse_integer_with_variant, unescape_text,
};
use crate::parser::{Parser, SpannedParserError};
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::pointer::PointerAddress;
use core::str::FromStr;

impl Parser {
    pub(crate) fn parse_atom(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {
            Token::LeftCurly => self.parse_map()?,
            Token::LeftBracket => self.parse_list()?,
            Token::TypeExpressionStart => {
                self.parse_wrapped_type_expression()?
            }
            Token::LeftParen => self.parse_parenthesized_statements()?,
            Token::If => self.parse_if_else()?,
            Token::Function | Token::Procedure => {
                self.parse_callable_definition()?
            }

            Token::Placeholder => self.parse_placeholder()?,

            Token::True => self.parse_true()?,
            Token::False => self.parse_false()?,
            Token::Null => self.parse_null()?,
            Token::Identifier(name) => self.parse_identifier(name)?,
            Token::NamedSlot(slot_name) => self.parse_named_slot(slot_name)?,
            Token::Slot(slot_address) => {
                self.parse_addressed_slot(slot_address)?
            }
            Token::PointerAddress(address) => {
                self.parse_pointer_address(address)?
            }
            Token::StringLiteral(value) => self.parse_string_literal(value)?,
            Token::Infinity => self.parse_infinity()?,
            Token::Nan => self.parse_nan()?,
            Token::Endpoint(endpoint_name) => {
                self.parse_endpoint(endpoint_name)?
            }
            Token::IntegerLiteral(integer_literal) => {
                self.parse_integer_literal(integer_literal)?
            }

            Token::BinaryIntegerLiteral(integer_with_variant)
            | Token::HexadecimalIntegerLiteral(integer_with_variant)
            | Token::OctalIntegerLiteral(integer_with_variant) => {
                self.parse_integer_with_variant(integer_with_variant)?
            }

            Token::DecimalLiteral(decimal_with_variant) => {
                self.parse_decimal_with_variant(decimal_with_variant)?
            }

            Token::FractionLiteral(fraction) => {
                self.parse_fraction_literal(fraction)?
            }

            _ => {
                return Err(SpannedParserError {
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
                    span: self.peek()?.span.clone(),
                });
            }
        })
    }

    pub(crate) fn parse_wrapped_type_expression(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        let start_token = self.advance()?;
        let type_expression = self.parse_type_expression(0);
        match type_expression {
            Ok(type_expression) => {
                // expect closing ')'
                let end_token = self.expect(Token::RightParen)?;
                let span = start_token.span.start..end_token.span.end;
                Ok(DatexExpressionData::TypeExpression(type_expression)
                    .with_span(span))
            }
            Err(e) => return self.collect_error_and_continue(e),
        }
    }

    pub(crate) fn parse_pointer_address(
        &mut self,
        address: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::PointerAddress(
            PointerAddress::try_from(&address[1..]).unwrap(),
        )
        .with_span(self.advance()?.span))
    }

    pub(crate) fn parse_integer_literal(
        &mut self,
        integer_literal: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        let span = self.advance()?.span.clone();
        Ok(match parse_integer_literal(integer_literal) {
            Ok(IntegerOrDecimal::Integer(integer)) => {
                DatexExpressionData::Integer(integer)
            }
            Ok(IntegerOrDecimal::TypedInteger(typed_integer)) => {
                DatexExpressionData::TypedInteger(typed_integer)
            }
            Ok(IntegerOrDecimal::Decimal(decimal)) => {
                DatexExpressionData::Decimal(decimal)
            }
            Ok(IntegerOrDecimal::TypedDecimal(typed_decimal)) => {
                DatexExpressionData::TypedDecimal(typed_decimal)
            }
            Err(e) => {
                return self.collect_error_and_continue(SpannedParserError {
                    error: ParserError::NumberParseError(e),
                    span,
                });
            }
        }
        .with_span(span))
    }

    pub(crate) fn parse_integer_with_variant(
        &mut self,
        integer_with_variant: IntegerWithVariant,
    ) -> Result<DatexExpression, SpannedParserError> {
        let token = self.advance()?;
        let span = token.span.clone();

        Ok(
            match parse_integer_with_variant(integer_with_variant, token.token)
            {
                Ok(IntegerOrTypedInteger::Integer(integer)) => {
                    DatexExpressionData::Integer(integer)
                }
                Ok(IntegerOrTypedInteger::TypedInteger(typed_integer)) => {
                    DatexExpressionData::TypedInteger(typed_integer)
                }
                Err(e) => {
                    return self.collect_error_and_continue(
                        SpannedParserError {
                            error: ParserError::NumberParseError(e),
                            span,
                        },
                    );
                }
            }
            .with_span(span),
        )
    }

    pub(crate) fn parse_decimal_with_variant(
        &mut self,
        decimal_with_variant: DecimalWithVariant,
    ) -> Result<DatexExpression, SpannedParserError> {
        let value = decimal_with_variant.value;
        let variant = decimal_with_variant.variant;

        let span = self.advance()?.span.clone();
        let res = match variant {
            Some(var) => {
                TypedDecimal::from_string_and_variant_in_range(&value, var)
                    .map(DatexExpressionData::TypedDecimal)
            }
            None => {
                Decimal::from_string(&value).map(DatexExpressionData::Decimal)
            }
        };
        match res {
            Ok(expr) => Ok(expr.with_span(span)),
            Err(e) => self.collect_error_and_continue(SpannedParserError {
                error: ParserError::NumberParseError(e),
                span,
            }),
        }
    }

    pub(crate) fn parse_placeholder(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Placeholder.with_span(self.advance()?.span))
    }

    pub(crate) fn parse_true(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Boolean(true).with_span(self.advance()?.span))
    }

    pub(crate) fn parse_false(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Boolean(false).with_span(self.advance()?.span))
    }

    pub(crate) fn parse_null(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Null.with_span(self.advance()?.span))
    }

    pub(crate) fn parse_identifier(
        &mut self,
        name: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Identifier(name)
            .with_span(self.advance()?.span))
    }

    pub(crate) fn parse_infinity(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Decimal(Decimal::Infinity)
            .with_span(self.advance()?.span))
    }

    pub(crate) fn parse_nan(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Decimal(Decimal::Nan)
            .with_span(self.advance()?.span))
    }

    pub(crate) fn parse_endpoint(
        &mut self,
        endpoint_name: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        let span = self.advance()?.span.clone();
        match Endpoint::from_str(endpoint_name.as_str()) {
            Err(e) => self.collect_error_and_continue(SpannedParserError {
                error: ParserError::InvalidEndpointName {
                    name: endpoint_name,
                    details: e,
                },
                span,
            }),
            Ok(endpoint) => {
                Ok(DatexExpressionData::Endpoint(endpoint).with_span(span))
            }
        }
    }

    pub(crate) fn parse_named_slot(
        &mut self,
        slot_name: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(
            DatexExpressionData::Slot(Slot::Named(slot_name[1..].to_string()))
                .with_span(self.advance()?.span),
        )
    }

    pub(crate) fn parse_addressed_slot(
        &mut self,
        slot_address: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(DatexExpressionData::Slot(Slot::Addressed(
            slot_address[1..].parse::<u32>().unwrap(),
        ))
        .with_span(self.advance()?.span))
    }

    pub(crate) fn parse_string_literal(
        &mut self,
        value: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        let unescaped = unescape_text(&value);
        Ok(
            DatexExpressionData::Text(unescaped)
                .with_span(self.advance()?.span),
        )
    }

    pub(crate) fn parse_fraction_literal(
        &mut self,
        fraction: String,
    ) -> Result<DatexExpression, SpannedParserError> {
        let span = self.advance()?.span.clone();
        // remove all underscores from fraction string
        let fraction: String = fraction.chars().filter(|&c| c != '_').collect();
        match Decimal::from_string(&fraction) {
            Ok(decimal) => {
                Ok(DatexExpressionData::Decimal(decimal).with_span(span))
            }
            Err(e) => self.collect_error_and_continue(SpannedParserError {
                error: ParserError::NumberParseError(e),
                span,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::expressions::{DatexExpressionData, Slot, Statements};
    use crate::ast::spanned::Spanned;
    use crate::ast::type_expressions::TypeExpressionData;
    use crate::ast::type_expressions::{TypeExpression, Union};
    use crate::parser::errors::ParserError;
    use crate::parser::parser_result::ParserResult;
    use crate::parser::tests::try_parse_and_return_on_first_error;
    use crate::parser::tests::{parse, try_parse_and_collect_errors};
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::endpoint::{
        Endpoint, InvalidEndpointError,
    };
    use crate::values::core_values::integer::typed_integer::TypedInteger;
    use crate::values::pointer::PointerAddress;
    use core::assert_matches::assert_matches;

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
        assert_eq!(
            expr.data,
            DatexExpressionData::Identifier("myVar".to_string())
        );
    }

    #[test]
    fn parse_string_literal() {
        let expr = parse("\"Hello, World!\"");
        assert_eq!(
            expr.data,
            DatexExpressionData::Text("Hello, World!".to_string())
        );

        let expr2 = parse("'Single quotes'");
        assert_eq!(
            expr2.data,
            DatexExpressionData::Text("Single quotes".to_string())
        );
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
        assert_eq!(
            expr.data,
            DatexExpressionData::Endpoint(Endpoint::new("@example"))
        );
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

        assert_eq!(
            ast.data,
            DatexExpressionData::Statements(Statements {
                statements: vec![
                    DatexExpressionData::Recover.with_default_span(),
                    DatexExpressionData::Boolean(true).with_default_span(),
                ],
                is_terminated: false,
                unbounded: None,
            })
        );
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
        assert_eq!(
            expr.data,
            DatexExpressionData::Slot(Slot::Named("mySlot".to_string()))
        );
    }

    #[test]
    fn parse_addressed_slot() {
        let expr = parse("#42");
        assert_eq!(expr.data, DatexExpressionData::Slot(Slot::Addressed(42)));
    }

    #[test]
    fn parse_pointer_address() {
        let expr = parse("$ABCDEF");
        assert_eq!(
            expr.data,
            DatexExpressionData::PointerAddress(
                PointerAddress::try_from("ABCDEF").unwrap()
            )
        );
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
        assert_eq!(
            expr.data,
            DatexExpressionData::TypedInteger(TypedInteger::I32(12345))
        );
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
        assert_eq!(
            expr.data,
            DatexExpressionData::TypedInteger(TypedInteger::U8(0xFF))
        );
    }

    #[test]
    fn parse_typed_binary_integer_literal() {
        let expr = parse("0b1010i16");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypedInteger(TypedInteger::I16(0b1010))
        );
    }

    #[test]
    fn parse_typed_octal_integer_literal() {
        let expr = parse("0o77u32");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypedInteger(TypedInteger::U32(0o77))
        );
    }

    #[test]
    fn parse_decimal_literal() {
        let expr = parse("123.456");
        assert_eq!(
            expr.data,
            DatexExpressionData::Decimal(
                Decimal::from_string("123.456").unwrap()
            )
        );
    }

    #[test]
    fn parse_typed_decimal_literal() {
        let expr = parse("78.9f32");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypedDecimal(TypedDecimal::F32(78.9.into()))
        );
    }

    #[test]
    fn parse_negative_decimal_literal() {
        let expr = parse("-0.001");
        assert_eq!(
            expr.data,
            DatexExpressionData::Decimal(
                Decimal::from_string("-0.001").unwrap()
            )
        );
    }

    #[test]
    fn parse_decimal_literal_exponent() {
        let expr = parse("1.23e4");
        assert_eq!(
            expr.data,
            DatexExpressionData::Decimal(
                Decimal::from_string("1.23e4").unwrap()
            )
        );
    }

    #[test]
    fn parse_typed_decimal_literal_exponent() {
        let expr = parse("5.67e-8f64");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypedDecimal(TypedDecimal::F64(
                5.67e-8.into()
            ))
        );
    }

    #[test]
    fn parse_int_decimal_literal_exponent() {
        let expr = parse("42e2");
        assert_eq!(
            expr.data,
            DatexExpressionData::Decimal(Decimal::from_string("42e2").unwrap())
        );
    }

    #[test]
    fn parse_typed_int_decimal_literal_exponent() {
        let expr = parse("100e3_f32");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypedDecimal(TypedDecimal::F32(100e3.into()))
        );
    }

    #[test]
    fn parse_fraction_literal() {
        let expr = parse("3/4");
        assert_eq!(
            expr.data,
            DatexExpressionData::Decimal(Decimal::from_string("3/4").unwrap())
        );
    }

    #[test]
    fn parse_fraction_literal_with_underscores() {
        let expr = parse("1_000/2_500");
        assert_eq!(
            expr.data,
            DatexExpressionData::Decimal(
                Decimal::from_string("1000/2500").unwrap()
            )
        );
    }

    #[test]
    fn parse_negative_fraction_literal_with_underscores() {
        let expr = parse("-7_500/2_500");
        assert_eq!(
            expr.data,
            DatexExpressionData::Decimal(
                Decimal::from_string("-7500/2500").unwrap()
            )
        );
    }

    #[test]
    fn parse_type_expression() {
        let expr = parse("type(1 | 2)");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypeExpression(
                TypeExpressionData::Union(Union(vec![
                    TypeExpressionData::Integer(1.into()).with_default_span(),
                    TypeExpressionData::Integer(2.into()).with_default_span()
                ]))
                .with_default_span()
            )
        );
    }
}
