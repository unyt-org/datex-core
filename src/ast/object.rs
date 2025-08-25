use crate::ast::error::pattern::Pattern;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::compiler::lexer::Token;

use chumsky::prelude::*;

pub fn object<'a>(
    key: impl DatexParserTrait<'a>,
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    key.then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(expression_without_tuple.clone())
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftCurly), just(Token::RightCurly))
        .map(DatexExpression::Object)
        .labelled(Pattern::Custom("object"))
        .as_context()
}
