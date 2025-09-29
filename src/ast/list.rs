use crate::ast::error::pattern::Pattern;
use crate::ast::lexer::Token;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use chumsky::prelude::*;

pub fn list<'a>(
    expression_without_list: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    expression_without_list
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
        .map(DatexExpression::List)
        .labelled(Pattern::List)
        .as_context()
}
