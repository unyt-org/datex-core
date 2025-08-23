use crate::ast::TokenInput;
use crate::compiler::lexer::Token;
use chumsky::extra::Err;
use chumsky::prelude::*;

pub fn whitespace<'a>()
-> impl Parser<'a, TokenInput<'a>, (), Err<Cheap>> + Clone + 'a {
    just(Token::Whitespace).repeated().ignored()
}

pub fn operation<'a>(
    c: Token,
) -> impl Parser<'a, TokenInput<'a>, Token, Err<Cheap>> + Clone + 'a {
    just(Token::Whitespace)
        .repeated()
        .at_least(1)
        .ignore_then(just(c))
        .then_ignore(just(Token::Whitespace).repeated().at_least(1))
}
