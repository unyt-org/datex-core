use crate::ast::grammar::utils::{operation, whitespace};
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::{DatexParserTrait, structs::expression::DatexExpressionData};
use chumsky::prelude::*;

use crate::ast::structs::expression::Range;

pub fn range<'a>(
    atomic: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    atomic
        .clone()
        .then(
            just(Token::Range)
                .padded_by(whitespace())
                .ignore_then(atomic),
        )
        .map_with(|(start, end), e| {
            DatexExpressionData::Range(Range {
                start: Box::new(start),
                end: Box::new(end),
            })
            .with_span(e.span())
        })
}

pub fn infix_range<'a>(
    atomic: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let base = atomic.clone();

    let tail = just(Token::Range)
        .padded_by(whitespace())
        .ignore_then(base.clone());

    base.foldl(tail.repeated(), |lhs, rhs| {
        let start = lhs.span.start.min(rhs.span.start);
        let end = lhs.span.start.min(rhs.span.end);
        DatexExpressionData::Range(Range {
            start: Box::new(lhs),
            end: Box::new(rhs),
        })
        .with_span(SimpleSpan::from(start..end))
    })
    .boxed()
}
