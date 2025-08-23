use crate::ast::DatexExpression;
use crate::ast::decimal::decimal;
use crate::ast::endpoint::endpoint;
use crate::ast::integer::integer;
use crate::ast::text::text;
use crate::compiler::lexer::Token;
use chumsky::extra::{Err, Full};
use chumsky::prelude::*;

/// A valid object / tuple key
/// (1: value), "key", 1, (("x"+"y"): 123)
pub fn key<'a>(
    wrapped_expression: impl Parser<
        'a,
        &'a [Token],
        DatexExpression,
        Full<Cheap, (), ()>,
    > + Clone
    + 'a,
) -> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    choice((
        text(),
        decimal(),
        integer(),
        endpoint(),
        // any valid identifiers (equivalent to variable names), mapped to a text
        select! {
            Token::Identifier(s) => DatexExpression::Text(s)
        },
        // dynamic key
        wrapped_expression.clone(),
    ))
}
