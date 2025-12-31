use std::str::FromStr;
use crate::values::core_values::decimal::Decimal;
use crate::parser::lexer::{DecimalWithVariant, IntegerWithVariant, Token};
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData};
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;
use crate::parser::utils::{parse_integer_literal, parse_integer_with_variant, unescape_text, IntegerOrDecimal, IntegerOrTypedInteger};
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;

impl Parser {
    pub(crate) fn parse_type_atom(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            Token::LeftCurly => self.parse_type_map()?,
            Token::LeftBracket => self.parse_type_list()?,
            Token::LeftParen => self.parse_type_grouped()?,
            
            Token::True => self.parse_type_true()?,
            Token::False => self.parse_type_false()?,
            Token::Null => self.parse_type_null()?,
            Token::Identifier(name) => self.parse_type_identifier(name)?,
            Token::StringLiteral(value) => self.parse_type_string_literal(value)?,
            Token::Infinity => self.parse_type_infinity()?,
            Token::Nan => self.parse_type_nan()?,
            Token::Endpoint(endpoint_name) => self.parse_type_endpoint(endpoint_name)?,
            Token::IntegerLiteral(integer_literal) => self.parse_type_integer_literal(integer_literal)?,
            Token::BinaryIntegerLiteral(integer_with_variant)
            | Token::HexadecimalIntegerLiteral(integer_with_variant)
            | Token::OctalIntegerLiteral(integer_with_variant) => {
                self.parse_type_integer_with_variant(integer_with_variant)?
            },
            Token::DecimalLiteral(decimal_with_variant) => {
                self.parse_type_decimal_literal(decimal_with_variant)?
            },
            Token::FractionLiteral(fraction) => {
                self.parse_type_fraction_literal(fraction)?
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
    
    pub(crate) fn parse_type_true(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(
            TypeExpressionData::Boolean(true)
                .with_span(self.advance()?.span)
        )
    }
    
    pub(crate) fn parse_type_false(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(
            TypeExpressionData::Boolean(false)
                .with_span(self.advance()?.span)
        )
    }
    
    pub(crate) fn parse_type_null(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(
            TypeExpressionData::Null
                .with_span(self.advance()?.span)
        )
    }
    
    pub(crate) fn parse_type_identifier(&mut self, name: String) -> Result<TypeExpression, SpannedParserError> {
        Ok(
            TypeExpressionData::Identifier(name)
                .with_span(self.advance()?.span)
        )
    }
    
    pub(crate) fn parse_type_string_literal(&mut self, value: String) -> Result<TypeExpression, SpannedParserError> {
        Ok(
            TypeExpressionData::Text(unescape_text(&value))
                .with_span(self.advance()?.span)
        )
    }
    
    pub(crate) fn parse_type_infinity(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(
            TypeExpressionData::Decimal(Decimal::Infinity)
                .with_span(self.advance()?.span)
        )
    }
    
    pub(crate) fn parse_type_nan(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(
            TypeExpressionData::Decimal(Decimal::Nan)
                .with_span(self.advance()?.span)
        )
    }
    
    pub(crate) fn parse_type_endpoint(&mut self, endpoint_name: String) -> Result<TypeExpression, SpannedParserError> {
        let span = self.advance()?.span.clone();
        match Endpoint::from_str(endpoint_name.as_str()) {
            Err(e) => Err(SpannedParserError {
                error: ParserError::InvalidEndpointName {name: endpoint_name, details: e},
                span,
            }),
            Ok(endpoint) => Ok(
                TypeExpressionData::Endpoint(endpoint).with_span(span)
            )
        }
    }
    
    pub(crate) fn parse_type_integer_literal(&mut self, integer_literal: String) -> Result<TypeExpression, SpannedParserError> {
        let span = self.advance()?.span.clone();
        Ok(
            match parse_integer_literal(integer_literal) {
                Ok(IntegerOrDecimal::Integer(integer)) => TypeExpressionData::Integer(integer),
                Ok(IntegerOrDecimal::TypedInteger(typed_integer)) => TypeExpressionData::TypedInteger(typed_integer),
                Ok(IntegerOrDecimal::Decimal(decimal)) => TypeExpressionData::Decimal(decimal),
                Ok(IntegerOrDecimal::TypedDecimal(typed_decimal)) => TypeExpressionData::TypedDecimal(typed_decimal),
                Err(e) => return self.collect_error_and_continue_with_type_expression(SpannedParserError {
                    error: ParserError::NumberParseError(e),
                    span,
                }),
            }.with_span(span)
        )
    }
    
    pub(crate) fn parse_type_integer_with_variant(&mut self, integer_with_variant: IntegerWithVariant) -> Result<TypeExpression, SpannedParserError> {
        let token = self.advance()?;
        let span = token.span.clone();
    
        Ok(
            match parse_integer_with_variant(integer_with_variant, token.token) {
                Ok(IntegerOrTypedInteger::Integer(integer)) => {
                    TypeExpressionData::Integer(integer)
                }
                Ok(IntegerOrTypedInteger::TypedInteger(typed_integer)) => {
                    TypeExpressionData::TypedInteger(typed_integer)
                }
                Err(e) => return self.collect_error_and_continue_with_type_expression(SpannedParserError {
                    error: ParserError::NumberParseError(e),
                    span,
                }),
            }.with_span(span)
        )
    }
    
    pub(crate) fn parse_type_decimal_literal(&mut self, decimal_with_variant: DecimalWithVariant) -> Result<TypeExpression, SpannedParserError> {
        let DecimalWithVariant { value, variant } = decimal_with_variant;
        
        let span = self.advance()?.span.clone();
        let res = match variant {
            Some(var) => TypedDecimal::from_string_and_variant_in_range(&value, var)
                .map(TypeExpressionData::TypedDecimal),
            None => Decimal::from_string(&value)
                .map(TypeExpressionData::Decimal),
        };
        match res {
            Ok(expr) => Ok(expr.with_span(span)),
            Err(e) => self.collect_error_and_continue_with_type_expression(SpannedParserError {
                error: ParserError::NumberParseError(e),
                span,
            }),
        }
    }
    
    pub(crate) fn parse_type_fraction_literal(&mut self, fraction: String) -> Result<TypeExpression, SpannedParserError> {
        let span = self.advance()?.span.clone();
        // remove all underscores from fraction string
        let fraction: String = fraction.chars().filter(|&c| c != '_').collect();
        match Decimal::from_string(&fraction) {
            Ok(decimal) => Ok(TypeExpressionData::Decimal(decimal).with_span(span)),
            Err(e) => self.collect_error_and_continue_with_type_expression(SpannedParserError {
                error: ParserError::NumberParseError(e),
                span,
            }),
        }
    }
    
}

#[cfg(test)]
mod tests {
    use datex_core::parser::parsers::type_expressions::tests::parse_type_expression;
    use crate::ast::structs::r#type::TypeExpressionData;
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::decimal::typed_decimal::{DecimalTypeVariant, TypedDecimal};
    use crate::values::core_values::endpoint::{Endpoint};
    use crate::values::core_values::integer::typed_integer::{IntegerTypeVariant, TypedInteger};

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

    #[test]
    fn parse_integer_literal() {
        let expr = parse_type_expression("12345");
        assert_eq!(expr.data, TypeExpressionData::Integer(12345.into()));
    }

    #[test]
    fn parse_typed_integer_literal() {
        let expr = parse_type_expression("12345u16");
        assert_eq!(expr.data, TypeExpressionData::TypedInteger(
            TypedInteger::from_string_with_variant("12345", IntegerTypeVariant::U16).unwrap()
        ));
    }

    #[test]
    fn parse_negative_integer_literal() {
        let expr = parse_type_expression("-6789");
        assert_eq!(expr.data, TypeExpressionData::Integer((-6789).into()));
    }

    #[test]
    fn parse_hexadecimal_integer_literal() {
        let expr = parse_type_expression("0x1A3F");
        assert_eq!(expr.data, TypeExpressionData::Integer(6719.into()));
    }

    #[test]
    fn parse_octal_integer_literal() {
        let expr = parse_type_expression("0o755");
        assert_eq!(expr.data, TypeExpressionData::Integer(493.into()));
    }

    #[test]
    fn parse_binary_integer_literal() {
        let expr = parse_type_expression("0b1101");
        assert_eq!(expr.data, TypeExpressionData::Integer(13.into()));
    }

    #[test]
    fn parse_decimal_literal() {
        let expr = parse_type_expression("123.456");
        assert_eq!(expr.data, TypeExpressionData::Decimal(Decimal::from_string("123.456").unwrap()));
    }

    #[test]
    fn parse_typed_decimal_literal() {
        let expr = parse_type_expression("123.456f32");
        assert_eq!(expr.data, TypeExpressionData::TypedDecimal(
            TypedDecimal::from_string_and_variant_in_range("123.456", DecimalTypeVariant::F32).unwrap()
        ));
    }
    
    #[test]
    fn parse_fraction_literal() {
        let expr = parse_type_expression("3/4");
        assert_eq!(expr.data, TypeExpressionData::Decimal(Decimal::from_string("3/4").unwrap()));
    }
}