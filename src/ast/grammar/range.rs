use crate::ast::grammar::utils::whitespace;
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
