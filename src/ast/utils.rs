use crate::ast::{DatexParserTrait, TokenInput};
use crate::compiler::lexer::Token;
use chumsky::extra::Err;
use chumsky::prelude::*;

pub fn whitespace<'a>() -> impl DatexParserTrait<'a, ()> {
    just(Token::Whitespace).repeated().ignored()
}

pub fn operation<'a>(c: Token) -> impl DatexParserTrait<'a, Token> {
    just(Token::Whitespace)
        .repeated()
        .at_least(0)
        .ignore_then(just(c))
        .then_ignore(just(Token::Whitespace).repeated().at_least(0))
}
