use crate::ast::error::pattern::Pattern;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::List;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;

pub fn list<'a>(
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    expression
        .clone()
        .separated_by(just(Token::Comma))
        .at_least(0)
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
        .map_with(|elements, e| {
            DatexExpressionData::List(List::new(elements)).with_span(e.span())
        })
        .labelled(Pattern::List)
        .as_context()
}
