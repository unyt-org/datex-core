use crate::ast::lexer::Token;
use crate::ast::r#type::r#type;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait, TokenInput, TypeExpression};
use chumsky::prelude::*;
use crate::ast::error::error::ParseError;

fn return_type<'a>() -> impl DatexParserTrait<'a, Option<TypeExpression>> {
    just(Token::Arrow)
        .padded_by(whitespace())
        .ignore_then(r#type().padded_by(whitespace()))
        .map(|ty| ty)
        .or_not()
}

fn body<'a>(
    statements: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    statements
        .clone()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
}


fn parameter<'a>() -> impl DatexParserTrait<'a, (String, TypeExpression)> {
    select! { Token::Identifier(name) => name }
        .then(
            just(Token::Colon)
                .padded_by(whitespace())
                .ignore_then(r#type().padded_by(whitespace())),
        )
        .map(|(name, ty)| (name, ty))
}

fn parameters<'a>() -> impl DatexParserTrait<'a, Vec<(String, TypeExpression)>> {
    parameter()
        .padded_by(whitespace())
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .allow_trailing()
        .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
        .padded_by(whitespace())
}

pub fn function<'a>(
    statements: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let function_body = body(statements);

    just(Token::Function)
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then(
            // FIXME why dis not working
            parameter()
            .padded_by(whitespace())
            .separated_by(just(Token::Comma).padded_by(whitespace()))
            .allow_trailing()
            .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
            .padded_by(whitespace()))
        .then(return_type())
        .then(function_body)
        .map(|(((name, params), return_type), body)| {
            DatexExpression::FunctionDeclaration {
                name,
                parameters: params,
                return_type,
                body: Box::new(body),
            }
        })
}
