use crate::ast::error::pattern::Pattern;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::compiler::lexer::Token;
use chumsky::prelude::*;

pub fn array<'a>(
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    expression_without_tuple
        .labelled(Pattern::Array)
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect::<Vec<_>>()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
        .map(DatexExpression::Array)
}
