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
                Some(var) => TypedDecimal::from_string_and_variant_in_range(&value, var).map(DatexExpression::TypedDecimal),
                None => Decimal::from_string(&value).map(DatexExpression::Decimal)
            }
        },
        Token::Nan => Ok(DatexExpression::Decimal(Decimal::NaN)),
        Token::Infinity(s) => Ok(DatexExpression::Decimal(
            if s.starts_with('-') {
                Decimal::NegInfinity
            } else {
                Decimal::Infinity
            }
        )),
        Token::FractionLiteral(s) => Decimal::from_string(&s).map(DatexExpression::Decimal),
    }.recover_invalid()
}
