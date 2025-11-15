use crate::ast::grammar::text::text;
use crate::ast::lexer::{IntegerLiteral, Token};
use crate::ast::spanned::Spanned;
use crate::ast::{DatexExpressionData, DatexParserTrait, ParserRecoverExt};
use crate::values::core_values::integer::Integer;
use chumsky::prelude::*;
/// A valid map key
/// abc, a, "1", "test", (1 + 2), ...
pub fn key<'a>(
    wrapped_expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        text(),
        // any valid identifiers (equivalent to variable names), mapped to a text
        select! {
             Token::Identifier(s) => DatexExpressionData::Text(s),
        }
        .map_with(|data, e| data.with_span(e.span())),
        // FIXME
        // select! {
        //     Token::DecimalIntegerLiteralWithVariant(IntegerLiteral { value, variant: None }) =>
        //         Integer::from_string(&value).map(DatexExpressionData::Integer),
        // }
        // .map_with(|data, e| data.map(|data| data.with_span(e.span())))
        // .recover_invalid(),
        // dynamic key
        wrapped_expression.clone(),
    ))
}
