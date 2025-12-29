use crate::ast::grammar::r#type::ty;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::FunctionDeclaration;
use crate::ast::structs::r#type::TypeExpression;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;
fn return_type<'a>() -> impl DatexParserTrait<'a, Option<TypeExpression>> {
    just(Token::Arrow)
        .ignore_then(ty())
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
                .ignore_then(ty()),
        )
        .map(|(name, ty)| (name, ty))
}

fn parameters<'a>() -> impl DatexParserTrait<'a, Vec<(String, TypeExpression)>>
{
    parameter()
        .separated_by(just(Token::Comma))
        .collect()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
}

pub fn function<'a>(
    statements: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    // TODO #358: support error notation

    just(Token::Function)
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
