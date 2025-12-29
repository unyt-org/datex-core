use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::{DatexExpression, DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;

pub fn operation<'a>(c: Token) -> impl DatexParserTrait<'a, Token> {
    just(c)
}
pub fn is_identifier(expr: &DatexExpression) -> bool {
    core::matches!(
        expr,
        DatexExpression {
            data: DatexExpressionData::Identifier { .. },
            ..
        }
    )
}
pub fn unwrap_single_statement(expr: DatexExpression) -> DatexExpression {
    match expr.data {
        DatexExpressionData::Statements(mut stmts) => {
            if stmts.statements.len() == 1 && !stmts.is_terminated {
                stmts.statements.remove(0)
            } else {
                DatexExpressionData::Statements(stmts).with_span(expr.span)
            }
        }
        _ => expr,
    }
}
