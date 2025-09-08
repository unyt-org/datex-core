use crate::ast::error::pattern::Pattern;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::compiler::lexer::Token;
use chumsky::prelude::*;

pub fn list<'a>(
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    expression_without_tuple
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::ListStart), just(Token::RightBracket))
        .map(DatexExpression::List)
        .labelled(Pattern::Array)
        .as_context()
    // just(Token::LeftBracket)
    //     .labelled(Pattern::Char('['))
    //     .ignore_then(
    //         expression_without_tuple
    //             .separated_by(just(Token::Comma).padded_by(whitespace()))
    //             .at_least(0)
    //             .allow_trailing()
    //             .collect::<Vec<_>>()
    //             .labelled(Pattern::Custom("array items")),
    //     )
    //     .padded_by(whitespace())
    //     .then_ignore(just(Token::RightBracket).labelled(Pattern::Char(']')))
    //     .map(DatexExpression::Array)
    //     .labelled(Pattern::Array)
    //     .as_context()
}
