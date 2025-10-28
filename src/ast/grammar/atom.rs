use crate::ast::DatexParserTrait;
use crate::ast::decimal::decimal;
use crate::ast::endpoint::endpoint;
use crate::ast::integer::integer;
use crate::ast::literal::literal;
use crate::ast::text::text;
use chumsky::prelude::*;

pub fn atom<'a>(
    list: impl DatexParserTrait<'a>,
    map: impl DatexParserTrait<'a>,
    wrapped_expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        list,
        map,
        literal(),
        decimal(),
        integer(),
        text(),
        endpoint(),
        wrapped_expression,
    ))
    .boxed()
}
