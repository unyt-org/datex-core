use crate::ast::utils::whitespace;
use crate::compiler::ast_parser::DatexExpression;
use crate::compiler::lexer::Token;
use chumsky::extra::{Err, Full};
use chumsky::prelude::*;

fn return_type<'a>(
    expression_without_tuple: impl Parser<
        'a,
        &'a [Token],
        DatexExpression,
        Full<Cheap, (), ()>,
    > + Clone
    + 'a,
) -> impl Parser<'a, &'a [Token], Option<DatexExpression>, Err<Cheap>> + Clone + 'a
{
    just(Token::Arrow)
        .padded_by(whitespace())
        .ignore_then(expression_without_tuple.padded_by(whitespace()))
        .or_not()
}

fn body<'a>(
    statements: impl Parser<'a, &'a [Token], DatexExpression, Full<Cheap, (), ()>>
    + Clone
    + 'a,
) -> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    statements
        .clone()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
}

fn parameters<'a>(
    tuple: impl Parser<'a, &'a [Token], DatexExpression, Full<Cheap, (), ()>>
    + Clone
    + 'a,
) -> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    tuple
        .or_not()
        .map(|e| e.unwrap_or(DatexExpression::Tuple(vec![])))
        .delimited_by(
            just(Token::LeftParen).padded_by(whitespace()), // '(' with spaces/newlines after
            just(Token::RightParen).padded_by(whitespace()), // ')' with spaces/newlines before
        )
}

pub fn function<'a>(
    statements: impl Parser<'a, &'a [Token], DatexExpression, Full<Cheap, (), ()>>
    + Clone
    + 'a,
    tuple: impl Parser<'a, &'a [Token], DatexExpression, Full<Cheap, (), ()>>
    + Clone
    + 'a,
    expression_without_tuple: impl Parser<
        'a,
        &'a [Token],
        DatexExpression,
        Full<Cheap, (), ()>,
    > + Clone
    + 'a,
) -> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    let function_params = parameters(tuple);
    let return_type = return_type(expression_without_tuple);
    let function_body = body(statements);
    just(Token::Function)
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then(function_params)
        .then(return_type)
        .then(function_body)
        .map(|(((name, params), return_type), body)| {
            DatexExpression::FunctionDeclaration {
                name,
                parameters: Box::new(params),
                return_type: return_type.map(Box::new),
                body: Box::new(body),
            }
        })
}
