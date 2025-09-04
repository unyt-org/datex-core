use crate::ast::unary_operation::UnaryOperator;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::compiler::lexer::Token;
use chumsky::prelude::*;

pub fn unary<'a>(atom: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    recursive(|unary| {
        // unary minus
        let negation =
            just(Token::Minus).then(unary.clone()).map(|(_, expr)| {
                DatexExpression::UnaryOperation(
                    UnaryOperator::Neg,
                    Box::new(expr),
                )
            });

        // logical NOT
        let logical_not =
            just(Token::Exclamation)
                .then(unary.clone())
                .map(|(_, expr)| {
                    DatexExpression::UnaryOperation(
                        UnaryOperator::Not,
                        Box::new(expr),
                    )
                });

        choice((negation, logical_not, atom))
    })
}
