use crate::ast::error::pattern::Pattern;
use crate::ast::lexer::Token;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;

pub fn list<'a>(
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    expression
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
        .map_with(|elements, e| {
            DatexExpressionData::List(elements).with_span(e.span())
        })
        .labelled(Pattern::List)
        .as_context()
}
