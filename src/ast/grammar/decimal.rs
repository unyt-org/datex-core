use crate::ast::DatexExpressionData;
use crate::ast::DatexParserTrait;
use crate::ast::ParserRecoverExt;
use crate::ast::lexer::{DecimalLiteral, Token};
use crate::ast::spanned::Spanned;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use chumsky::prelude::*;
pub fn decimal<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::DecimalLiteralWithSuffix(DecimalLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedDecimal::from_string_and_variant_in_range(&value, var).map(DatexExpressionData::TypedDecimal),
                None => Decimal::from_string(&value).map(DatexExpressionData::Decimal)
            }
        },
        Token::Nan => Ok(DatexExpressionData::Decimal(Decimal::NaN)),
        Token::Infinity => Ok(DatexExpressionData::Decimal(Decimal::Infinity)),
        Token::FractionLiteral(s) => Decimal::from_string(&s).map(DatexExpressionData::Decimal),
    }
        .map_with(|data, e| data.map(|data| data.with_span(e.span())))
        .recover_invalid()
}
