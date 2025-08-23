use crate::ast::DatexParserTrait;
use crate::ast::decimal::decimal;
use crate::ast::endpoint::endpoint;
use crate::ast::integer::integer;
use crate::ast::literal::literal;
use crate::ast::text::text;
use chumsky::prelude::*;

pub fn atom<'a>(
    array: impl DatexParserTrait<'a>,
    object: impl DatexParserTrait<'a>,
    wrapped_expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
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
