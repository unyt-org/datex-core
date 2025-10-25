use crate::ast::lexer::Token;
use crate::ast::tree::{FunctionDeclaration, TypeExpressionData};
use crate::ast::r#type::r#type;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;

fn return_type<'a>() -> impl DatexParserTrait<'a, Option<TypeExpressionData>> {
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

fn parameter<'a>() -> impl DatexParserTrait<'a, (String, TypeExpressionData)> {
    select! { Token::Identifier(name) => name }
        .then(
            just(Token::Colon)
                .padded_by(whitespace())
                .ignore_then(r#type().padded_by(whitespace())),
        )
        .map(|(name, ty)| (name, ty))
}

fn parameters<'a>() -> impl DatexParserTrait<'a, Vec<(String, TypeExpressionData)>>
{
    parameter()
        .padded_by(whitespace())
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
        .padded_by(whitespace())
}

pub fn function<'a>(
    statements: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    // TODO #358: support error notation

    just(Token::Function)
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then(parameters())
        .then(return_type())
        .then(body(statements))
        .map_with(|(((name, params), return_type), body), e| {
            DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
                name,
                parameters: params,
                return_type,
                body: Box::new(body),
            })
            .with_span(e.span())
        })
}
