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

pub fn decimal<'a>() -> impl DatexParserTrait<'a> {
    let numeric_token = select! {
        Token::DecimalNumericLiteral(parts) => parts
    };
    let integer_token = select! {
        Token::DecimalNumericLiteral(NumericLiteralParts { integer_part, exponent_part: None, variant_part: None  }) => integer_part
    };
    let build_from_parts = move |left: NumericLiteralParts,
                                 opt_right: Option<NumericLiteralParts>,
                                 dot_present: bool|
          -> Result<
        DatexExpressionData,
        NumberParseError,
    > {
        // helper to handle typed variant if present
        let make_typed_or_plain =
            |value: String,
             variant_opt: Option<String>|
             -> Result<DatexExpressionData, NumberParseError> {
                match variant_opt {
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
            };

        if dot_present {
            match opt_right {
                Some(right) => {
                    let mut s = left.integer_part;
                    s.push('.');
                    s.push_str(&right.integer_part);
                    if let Some(exp) = right.exponent_part {
                        s.push('e');
                        s.push_str(&exp);
                    }
                    // prefer variant present on the fractional side
                    make_typed_or_plain(s, right.variant_part)
                }
                None => {
                    // `42.`is mapped to `42.0`
                    let mut s = left.integer_part;
                    s.push_str(".0");
                    make_typed_or_plain(s, left.variant_part)
                }
            }
        } else {
            // no dot present, maybe exponent inside left, maybe variant
            let mut s = left.integer_part;
            if let Some(exp) = left.exponent_part {
                s.push('e');
                s.push_str(&exp);
            }
            make_typed_or_plain(s, left.variant_part)
        }
    };

    choice((
        // special tokens
        select! {
            Token::Nan => Ok(DatexExpressionData::Decimal(Decimal::Nan)),
            Token::Infinity => Ok(DatexExpressionData::Decimal(Decimal::Infinity)),
        },

        // exponent only, no dot: <num> 'e' <exponent>
        select! {
            Token::DecimalNumericLiteral(NumericLiteralParts { integer_part, exponent_part: Some(exp), variant_part  }) => (
                integer_part,
                exp,
                variant_part
            )
        }.map(|(integer_part, exponent_part, variant_part)| {
            let mut value = integer_part;
            value.push('e');
            value.push_str(&exponent_part);
            match variant_part {
                Some(var) => {
                    let variant = DecimalTypeVariant::from_str(&var)
                        .map_err(|_| NumberParseError::InvalidFormat)?;
                    TypedDecimal::from_string_and_variant_in_range(
                        &value, variant,
                    ).map(DatexExpressionData::TypedDecimal)
                }
                None => Decimal::from_string(&value).map(DatexExpressionData::Decimal),
            }
        }),

        // fraction: <num> '/' <denom>
        integer_token
            .then_ignore(just(Token::Slash))
            .then(integer_token)
            .map(|(num, denom)| {
                let s = format!("{}/{}", num, denom);
                Decimal::from_string(&s).map(DatexExpressionData::Decimal)
            }),

        // prefix shortcut: '.' <digits> 
        // mappes to: 0.<digits>
        just(Token::Dot)
            .then(numeric_token)
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
                        ).map(DatexExpressionData::TypedDecimal)
                    }
                    None => Decimal::from_string(&value).map(DatexExpressionData::Decimal),
                }
            }),

        // e-notation on base with dot: <digits> '.' 'e' <exponent>
        // e.g., 1.e2, 42.e-3
        numeric_token
            .then_ignore(just(Token::Dot))
            .then(select! {
                Token::Identifier(id) => id
            })
            .map(|(left, exponent_part)| {
                // identifier must start with 'e' or 'E' (otherwise it's not e-notation)
                if exponent_part.starts_with('e') || exponent_part.starts_with('E') {
                    let mut value = left.integer_part;
                    value.push('.');
                    value.push_str(&exponent_part);
                    Decimal::from_string(&value).map(DatexExpressionData::Decimal)
                } else {
                    Err(NumberParseError::InvalidFormat)
                }
            }),
        // ordinary decimal number parsing with dot:
        // - `42e2` -> exponent inside token
        // - `42.`  -> dot present, no fractional token -> suffix shortcut -> 42.0
        // - `42.5` -> dot + fractional token
        numeric_token
            .then(
               choice((
                    // dot + fractional token
                    just(Token::Dot)
                        .ignore_then(numeric_token)
                        .map(Some),
                    // dot suffix, no fractional token
                    just(Token::Dot).map(|_| None),
                ))
            )
            .map(move |(left, opt_dot_fraction)| {
                match opt_dot_fraction {
                    None => build_from_parts(left, None, false),
                    Some(maybe_right) => build_from_parts(left, Some(maybe_right), true),
                }
            }),

    ))
    .map_with(|data, e| data.map(|data| data.with_span(e.span())))
    .recover_invalid()
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{
            grammar::decimal::decimal, parse_with_parser, spanned::Spanned,
            structs::expression::DatexExpressionData,
        },
        values::core_values::decimal::{
            Decimal,
            typed_decimal::{DecimalTypeVariant, TypedDecimal},
        },
    };

    fn parse_decimal(src: &str) -> crate::ast::DatexParseResult {
        parse_with_parser(src, decimal())
    }

    fn ensure_invalid(src: &str) {
        let res = parse_decimal(src);
        assert!(!res.is_valid(), "Expected error for input: {}", src);
    }

    #[test]
    fn simple() {
        let src = "3.41";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(Decimal::from_string("3.41").unwrap())
                .with_default_span()
        );
    }

    #[test]
    fn special() {
        let src = "nan";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(Decimal::from_string("nan").unwrap())
                .with_default_span()
        );

        let src = "infinity";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("infinity").unwrap()
            )
            .with_default_span()
        );
    }

    #[test]
    fn typed() {
        let src = "2.71828f64";
        let num = parse_decimal(src).unwrap().ast;
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

        let src = "1.618033f32";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::TypedDecimal(
                TypedDecimal::from_string_and_variant_in_range(
                    "1.618033",
                    DecimalTypeVariant::F32,
                )
                .unwrap()
            )
            .with_default_span()
        );
    }

    #[test]
    fn shortcut_prefix() {
        // no variant and no exponent
        let src = ".57721";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("0.57721").unwrap()
            )
            .with_default_span()
        );

        // with variant
        let src = ".314f32";
        let num = parse_decimal(src).unwrap().ast;
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
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("0.159e2").unwrap()
            )
            .with_default_span()
        );

        // with variant and exponent
        let src = ".265e-3f64";
        let num = parse_decimal(src).unwrap().ast;
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
    fn shortcut_suffix() {
        let src = "42.";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(Decimal::from_string("42.0").unwrap())
                .with_default_span()
        );
    }

    #[test]
    fn fraction() {
        let src = "3/4";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(Decimal::from_string("0.75").unwrap())
                .with_default_span()
        );
    }

    #[test]
    fn exponent() {
        // only exponent, no dot
        let src = "2e10";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("20000000000.0").unwrap()
            )
            .with_default_span()
        );

        let src = "2e-2";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(Decimal::from_string("0.02").unwrap())
                .with_default_span()
        );

        // positive exponent
        let src = "1.23e4";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("1.23e4").unwrap()
            )
            .with_default_span()
        );

        // negative exponent
        let src = "5.67e-3";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("5.67e-3").unwrap()
            )
            .with_default_span()
        );

        // variant with exponent
        let src = "9.81e2f32";
        let num = parse_decimal(src).unwrap().ast;
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

        // with exponent
        let src = "7.e3";
        let num = parse_decimal(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("7000.0").unwrap()
            )
            .with_default_span()
        );
    }

    #[test]
    fn invalid_cases() {
        let cases = vec![
            "42", // no dot
            // "42e1.",   // dot after exponent FIXME: Shall we disallow this?
            "3.14.15", // multiple dots
            "2..718",  // multiple dots
            "1.0f128", // invalid variant
            "4.2e2.5", // invalid exponent format
            "5.67e",   // missing exponent value
            ".e3",     // missing base digits
            "3/0",     // denom zero
            "abc",     // not a number
        ];

        for src in cases {
            ensure_invalid(src);
        }
    }
}
