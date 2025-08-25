use crate::ast::error::error::ParseError;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait, ParserRecoverExt};
use crate::compiler::lexer::Token;
use chumsky::util::Maybe;
use chumsky::{DefaultExpected, prelude::*, recovery};

pub fn array<'a>(
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let items = expression_without_tuple
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .allow_trailing()
        .collect::<Vec<_>>()
        .padded_by(whitespace());

    // Expect the closing ']' but recover if it's missing
    let close = just(Token::RightBracket)
        .to(()) // make the output `()`, simpler to fake on recovery
        .recover_with(recovery::via_parser(recovery::nested_delimiters(
            Token::LeftBracket,
            Token::RightBracket,
            [(Token::LeftParen, Token::RightParen)],
            |_| (), // fallback value when we had to recover
        )))
        .or_not(); // still succeed so we can build the Array node

    just(Token::LeftBracket)
        .ignore_then(items)
        .then(close)
        .map(|(elems, _)| DatexExpression::Array(elems))
}
