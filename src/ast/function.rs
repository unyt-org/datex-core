use crate::ast::lexer::Token;
use crate::ast::r#type::r#type;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait, TypeExpression};
use chumsky::prelude::*;

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

// TODO don't use map here, custom syntax for function params
fn parameters<'a>(
    r#map: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    r#map
        .clone()
        .or_not()
        .map(|e| e.unwrap_or(DatexExpression::Map(vec![])))
        .delimited_by(
            just(Token::LeftParen).padded_by(whitespace()), // '(' with spaces/newlines after
            just(Token::RightParen).padded_by(whitespace()), // ')' with spaces/newlines before
        )
}

pub fn function<'a>(
    statements: impl DatexParserTrait<'a>,
    map: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let function_params = parameters(map);
    let function_body = body(statements);
    just(Token::Function)
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then(function_params)
        .then(return_type())
        .then(function_body)
        .map(|(((name, params), return_type), body)| {
            DatexExpression::FunctionDeclaration {
                name,
                parameters: Box::new(params),
                return_type,
                body: Box::new(body),
            }
        })
}
