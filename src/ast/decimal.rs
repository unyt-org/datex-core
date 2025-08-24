use crate::ast::DatexExpression;
use crate::ast::DatexParserTrait;
use crate::ast::ParserRecoverExt;
use crate::compiler::lexer::{DecimalLiteral, Token};
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use chumsky::prelude::*;

pub fn decimal<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::DecimalLiteral(DecimalLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedDecimal::from_string_with_variant(&value, var).map(DatexExpression::TypedDecimal),
                None => Ok(DatexExpression::Decimal(Decimal::from_string(&value)))
            }
        },
        Token::NanLiteral => Ok(DatexExpression::Decimal(Decimal::NaN)),
        Token::InfinityLiteral(s) => Ok(DatexExpression::Decimal(
            if s.starts_with('-') {
                Decimal::NegInfinity
            } else {
                Decimal::Infinity
            }
        )),
        Token::FractionLiteral(s) => Ok(DatexExpression::Decimal(Decimal::from_string(&s))),
    }.recover_invalid()
}
