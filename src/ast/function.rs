use crate::ast::lexer::Token;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use chumsky::prelude::*;

fn return_type<'a>(
    expression_without_list: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a, Option<DatexExpression>> {
    just(Token::Arrow)
        .padded_by(whitespace())
        .ignore_then(expression_without_list.padded_by(whitespace()))
        .or_not()
}

fn body<'a>(
    statements: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    statements
        .clone()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
}

fn parameters<'a>(
    r#struct: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    r#struct
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
    r#struct: impl DatexParserTrait<'a>,
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let function_params = parameters(r#struct);
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
