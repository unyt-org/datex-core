use crate::ast::DatexExpression;
use crate::ast::TokenInput;
use crate::compiler::lexer::{DecimalLiteral, Token};
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use chumsky::extra::Err;
use chumsky::prelude::*;

pub fn decimal<'a>()
-> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a {
    select! {
        Token::DecimalLiteral(DecimalLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedDecimal::from_string_with_variant(&value, var)
                    .map(DatexExpression::TypedDecimal)
                    .unwrap_or(DatexExpression::Invalid),
                None => DatexExpression::Decimal(Decimal::from_string(&value))
            }
        },
        Token::NanLiteral => DatexExpression::Decimal(Decimal::NaN),
        Token::InfinityLiteral(s) => DatexExpression::Decimal(
            if s.starts_with('-') {
                Decimal::NegInfinity
            } else {
                Decimal::Infinity
            }
        ),
        Token::FractionLiteral(s) => DatexExpression::Decimal(Decimal::from_string(&s)),
    }
}
