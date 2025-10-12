use crate::ast::error::pattern::Pattern;
use crate::ast::lexer::Token;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpressionData, DatexParserTrait};

use chumsky::prelude::*;

pub fn map<'a>(
    key: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    key.then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(expression.clone())
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftCurly), just(Token::RightCurly))
        .map_with(|entries, e| {
            DatexExpressionData::Map(entries).with_span(e.span())
        })
        .labelled(Pattern::Custom("map"))
        .as_context()
}
