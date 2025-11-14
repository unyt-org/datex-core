use crate::ast::DatexExpressionData;
use crate::ast::DatexParserTrait;
use crate::ast::ParserRecoverExt;
use crate::ast::lexer::{IntegerLiteral, Token};
use crate::ast::spanned::Spanned;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use chumsky::prelude::*;

pub fn integer<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::DecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_with_variant(&value, var)
                    .map(DatexExpressionData::TypedInteger),
                None => Integer::from_string(&value)
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
