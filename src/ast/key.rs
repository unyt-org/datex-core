use crate::ast::lexer::Token;
use crate::ast::text::text;
use crate::ast::{DatexExpressionData, DatexParserTrait};
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
            Token::Identifier(s) => DatexExpressionData::Text(s)
        }.map_with(|data, e| data.with_span(e.span())),
        // dynamic key
        wrapped_expression.clone(),
    ))
}
