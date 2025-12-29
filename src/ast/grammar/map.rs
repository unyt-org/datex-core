use crate::ast::error::pattern::Pattern;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::{DatexExpressionData, DatexParserTrait};

use crate::ast::structs::expression::Map;
use chumsky::prelude::*;

pub fn map<'a>(
    key: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    key.then_ignore(just(Token::Colon))
        .then(expression.clone())
        .separated_by(just(Token::Comma))
        .at_least(0)
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::LeftCurly), just(Token::RightCurly))
        .map_with(|entries, e| {
            DatexExpressionData::Map(Map::new(entries)).with_span(e.span())
        })
        .labelled(Pattern::Custom("map"))
        .as_context()
}
