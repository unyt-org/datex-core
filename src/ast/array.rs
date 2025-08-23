use crate::ast::utils::whitespace;
use crate::compiler::ast_parser::DatexExpression;
use crate::compiler::lexer::Token;
use chumsky::extra::{Err, Full};
use chumsky::prelude::*;
use chumsky::recursive::Indirect;

pub fn array<'a>(
    expression_without_tuple: Recursive<
        Indirect<'a, 'a, &'a [Token], DatexExpression, Full<Cheap, (), ()>>,
    >,
) -> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    expression_without_tuple
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
        .map(DatexExpression::Array)
}
