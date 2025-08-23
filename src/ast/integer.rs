use crate::ast::DatexExpression;
use crate::ast::DatexParserTrait;
use crate::ast::TokenInput;
use crate::compiler::lexer::{IntegerLiteral, Token};
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use chumsky::extra::Err;
use chumsky::prelude::*;

pub fn integer<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::DecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_with_variant(&value, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string(&value)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
        Token::BinaryIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 2, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string_radix(&value[2..], 2)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
        Token::HexadecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 16, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string_radix(&value[2..], 16)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
        Token::OctalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 8, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string_radix(&value[2..], 8)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
    }
}
