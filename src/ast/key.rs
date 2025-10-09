use crate::ast::lexer::Token;
use crate::ast::text::text;
use crate::ast::{DatexExpression, DatexParserTrait};
use chumsky::prelude::*;

/// A valid map key
/// (1: value), "key", 1, (("x"+"y"): 123)
pub fn key<'a>(
    wrapped_expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        text(),
        // any valid identifiers (equivalent to variable names), mapped to a text
        select! {
            Token::Identifier(s) => DatexExpression::Text(s)
        },
        // dynamic key
        wrapped_expression.clone(),
    ))
}
