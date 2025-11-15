use core::str::FromStr;

use crate::ast::DatexExpressionData;
use crate::ast::DatexParserTrait;
use crate::ast::ParserRecoverExt;
use crate::ast::lexer::NumericLiteralParts;
use crate::ast::lexer::{IntegerLiteral, Token};
use crate::ast::spanned::Spanned;
use crate::values::core_values::error::NumberParseError;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use chumsky::prelude::*;

pub fn integer<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::DecimalNumericLiteral(NumericLiteralParts { integer_part, exponent_part: None, variant_part }) => {
            match variant_part {
                Some(var) => {
                    let variant = IntegerTypeVariant::from_str(&var);
                    if variant.is_err() {
                        return Some(Err(NumberParseError::InvalidFormat));
                    }
                    let variant = variant.unwrap();
                    TypedInteger::from_string_with_variant(&integer_part, variant)
                    .map(DatexExpressionData::TypedInteger)
                },
                None => Integer::from_string(&integer_part)
                    .map(DatexExpressionData::Integer),
            }
        },
        Token::BinaryIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 2, var)
                    .map(DatexExpressionData::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 2)
                    .map(DatexExpressionData::Integer),
            }
        },
        Token::HexadecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 16, var)
                    .map(DatexExpressionData::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 16)
                    .map(DatexExpressionData::Integer),
            }
        },
        Token::OctalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 8, var)
                    .map(DatexExpressionData::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 8)
                    .map(DatexExpressionData::Integer),
            }
        },
    }
        .map_with(|data, e| data.map(|data| data.with_span(e.span())))
        .recover_invalid()
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{
            parse, spanned::Spanned, structs::expression::DatexExpressionData,
        },
        values::core_values::integer::{
            Integer,
            typed_integer::{IntegerTypeVariant, TypedInteger},
        },
    };

    #[test]
    fn simple() {
        let src = "0";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Integer(Integer::from_string("0").unwrap())
                .with_default_span()
        );

        let src = "123456789123456789";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Integer(
                Integer::from_string("123456789123456789").unwrap()
            )
            .with_default_span()
        );
    }

    #[test]
    fn with_underscores() {
        let src = "123_456_789_123_456_789";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::Integer(
                Integer::from_string("123456789123456789").unwrap()
            )
            .with_default_span()
        );
    }

    #[test]
    fn with_variant() {
        let src = "123456789u32";
        let num = parse(src).unwrap().ast;
        assert_eq!(
            num,
            DatexExpressionData::TypedInteger(
                TypedInteger::from_string_with_variant(
                    "123456789",
                    IntegerTypeVariant::U32
                )
                .unwrap()
            )
            .with_default_span()
        );
    }
}
