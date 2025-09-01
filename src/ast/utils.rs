use crate::ast::{DatexExpression, DatexParserTrait};
use crate::compiler::lexer::Token;
use chumsky::prelude::*;

pub fn whitespace<'a>() -> impl DatexParserTrait<'a, ()> {
    just(Token::Whitespace).repeated().ignored()
}

pub fn operation<'a>(c: Token) -> impl DatexParserTrait<'a, Token> {
    just(Token::Whitespace)
        .repeated()
        .ignore_then(just(c))
        .then_ignore(just(Token::Whitespace).repeated())
}
pub fn is_literal(expr: &DatexExpression) -> bool {
    match expr {
        DatexExpression::Literal { .. } => true,
        _ => false,
    }
}
pub fn unwrap_single_statement(expr: DatexExpression) -> DatexExpression {
    match expr {
        DatexExpression::Statements(mut stmts) => {
            if stmts.len() == 1 && stmts[0].is_terminated {
                stmts.remove(0).expression
            } else {
                DatexExpression::Statements(stmts)
            }
        }
        other => other,
    }
}
