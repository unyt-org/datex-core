use crate::ast::DatexParserTrait;
use crate::ast::grammar::decimal::decimal;
use crate::ast::grammar::endpoint::endpoint;
use crate::ast::grammar::integer::integer;
use crate::ast::grammar::literal::literal;
use crate::ast::grammar::text::text;
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
