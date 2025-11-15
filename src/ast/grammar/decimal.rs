use core::str::FromStr;

use crate::ast::DatexExpressionData;
use crate::ast::DatexParserTrait;
use crate::ast::ParserRecoverExt;
use crate::ast::lexer::NumericLiteralParts;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::error::NumberParseError;
use chumsky::prelude::*;

/// Parses the base part of a decimal number (the part before the dot).
fn decimal_base<'a>() -> impl DatexParserTrait<'a, String> {
    select! {
        Token::DecimalNumericLiteral(NumericLiteralParts {exponent_part: None, integer_part, variant_part: None}) => {
           integer_part
        },
    }
}

/// Parses a fractional decimal number (e.g., "3/4") into a `Decimal`.
fn fraction<'a>()
-> impl DatexParserTrait<'a, Result<DatexExpressionData, NumberParseError>> {
    select! {
        Token::DecimalNumericLiteral(NumericLiteralParts { exponent_part: None, integer_part: left, variant_part: None }) => left
    }
    .then_ignore(select! { Token::Slash => () })
    .then(
        select! {
            Token::DecimalNumericLiteral(NumericLiteralParts { exponent_part: None, integer_part: right, variant_part: None }) => right
        }
    )
    .map(|(num, denom)| {
        let s = format!("{}/{}", num, denom);
        Decimal::from_string(&s).map(DatexExpressionData::Decimal)
    })
}

/// Parses a default decimal number (with a dot) into a `Decimal` or `TypedDecimal`.
/// For example: "3.14", "2.71828f64"
fn default_decimal<'a>()
-> impl DatexParserTrait<'a, Result<DatexExpressionData, NumberParseError>> {
    decimal_base()
        .then_ignore(just(Token::Dot))
        .then(select! {
            Token::DecimalNumericLiteral(parts) => parts
        })
        .map(|(left, right)| {
            let mut value = left;
            value.push('.');
            value.push_str(&right.integer_part);
            if let Some(exp) = right.exponent_part {
                value.push('e');
                value.push_str(&exp);
            }
            match right.variant_part {
                Some(var) => {
                    let variant = DecimalTypeVariant::from_str(&var)
                        .map_err(|_| NumberParseError::InvalidFormat)?;
                    TypedDecimal::from_string_and_variant_in_range(
                        &value, variant,
                    )
                    .map(DatexExpressionData::TypedDecimal)
                }
                None => Decimal::from_string(&value)
                    .map(DatexExpressionData::Decimal),
            }
        })
    // .map_with(|data, e| data.map(|data| data.with_span(e.span())))
    // .recover_invalid()
}

fn shortcut_decimal<'a>()
-> impl DatexParserTrait<'a, Result<DatexExpressionData, NumberParseError>> {
    just(Token::Dot)
        .then(select! {
            Token::DecimalNumericLiteral(parts) => parts
        })
        .map(|(_, right)| {
            let mut value = String::from("0.");
            value.push_str(&right.integer_part);
            if let Some(exp) = right.exponent_part {
                value.push('e');
                value.push_str(&exp);
            }
            match right.variant_part {
                Some(var) => {
                    let variant = DecimalTypeVariant::from_str(&var)
                        .map_err(|_| NumberParseError::InvalidFormat)?;
                    TypedDecimal::from_string_and_variant_in_range(
                        &value, variant,
                    )
                    .map(DatexExpressionData::TypedDecimal)
                }
                None => Decimal::from_string(&value)
                    .map(DatexExpressionData::Decimal),
            }
        })
}

pub fn decimal<'a>() -> impl DatexParserTrait<'a> {
    choice((
        select! {
            Token::Nan => Ok(DatexExpressionData::Decimal(Decimal::NaN)),
            Token::Infinity => Ok(DatexExpressionData::Decimal(Decimal::Infinity)),
        },
        shortcut_decimal(),
        fraction(),
        default_decimal(),
    ))
        .map_with(|data, e| data.map(|data| data.with_span(e.span())))
        .recover_invalid()
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{
            parse, spanned::Spanned, structs::expression::DatexExpressionData,
        },
        values::core_values::decimal::{
            Decimal,
            typed_decimal::{DecimalTypeVariant, TypedDecimal},
        },
    };

    #[test]
    fn simple() {
        let src = "3.41";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(Decimal::from_string("3.41").unwrap())
                .with_default_span()
        );
    }

    #[test]
    fn typed() {
        let src = "2.71828f64";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::TypedDecimal(
                TypedDecimal::from_string_and_variant_in_range(
                    "2.71828",
                    DecimalTypeVariant::F64,
                )
                .unwrap()
            )
            .with_default_span()
        );
    }

    #[test]
    fn shortcut() {
        // no variant and no exponent
        let src = ".57721";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("0.57721").unwrap()
            )
            .with_default_span()
        );

        // with variant
        let src = ".314f32";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::TypedDecimal(
                TypedDecimal::from_string_and_variant_in_range(
                    "0.314",
                    DecimalTypeVariant::F32,
                )
                .unwrap()
            )
            .with_default_span()
        );

        // with exponent
        let src = ".159e2";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("0.159e2").unwrap()
            )
            .with_default_span()
        );

        // with variant and exponent
        let src = ".265e-3f64";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::TypedDecimal(
                TypedDecimal::from_string_and_variant_in_range(
                    "0.265e-3",
                    DecimalTypeVariant::F64,
                )
                .unwrap()
            )
            .with_default_span()
        );
    }

    #[test]
    fn fraction() {
        let src = "3/4";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(Decimal::from_string("0.75").unwrap())
                .with_default_span()
        );
    }

    #[test]
    fn exponent() {
        // positive exponent
        let src = "1.23e4";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("1.23e4").unwrap()
            )
            .with_default_span()
        );

        // negative exponent
        let src = "5.67e-3";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("5.67e-3").unwrap()
            )
            .with_default_span()
        );

        // variant with exponent
        let src = "9.81e2f32";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::TypedDecimal(
                TypedDecimal::from_string_and_variant_in_range(
                    "9.81e2",
                    DecimalTypeVariant::F32,
                )
                .unwrap()
            )
            .with_default_span()
        );
    }
}
