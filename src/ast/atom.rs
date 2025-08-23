use crate::ast::DatexExpression;
use crate::ast::TokenInput;
use crate::ast::decimal::decimal;
use crate::ast::endpoint::endpoint;
use crate::ast::integer::integer;
use crate::ast::literal::literal;
use crate::ast::text::text;
use crate::compiler::lexer::Token;
use chumsky::extra::{Err, Full};
use chumsky::prelude::*;

pub fn atom<'a>(
    array: impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a,
    object: impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>>
    + Clone
    + 'a,
    wrapped_expression: impl Parser<
        'a,
        &'a [Token],
        DatexExpression,
        Full<Cheap, (), ()>,
    > + Clone
    + 'a,
) -> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a {
    choice((
        array,
        object,
        literal(),
        decimal(),
        integer(),
        text(),
        endpoint(),
        wrapped_expression,
    ))
    .boxed()
}
