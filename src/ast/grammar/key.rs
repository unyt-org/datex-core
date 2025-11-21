use crate::ast::grammar::integer::integer_base;
use crate::ast::grammar::text::text;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;
/// A valid map key
/// abc, a, "1", "test", (1 + 2), 5, ...
pub fn key<'a>(
    wrapped_expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        text(),
        integer_base(),
        // any valid identifiers (equivalent to variable names), mapped to a text
        select! {
             Token::Identifier(s) => DatexExpressionData::Text(s),
        }
        .map_with(|data, e| data.with_span(e.span())),
        // dynamic key
        wrapped_expression.clone(),
    ))
}
