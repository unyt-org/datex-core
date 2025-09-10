use crate::ast::unary_operation::UnaryOperator;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::ast::lexer::Token;
use chumsky::prelude::*;

pub fn unary<'a>(atom: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    recursive(|unary| {
        // unary minus
        let minus = just(Token::Minus).then(unary.clone()).map(|(_, expr)| {
            DatexExpression::UnaryOperation(
                UnaryOperator::Minus,
                Box::new(expr),
            )
        });
        // unary plus
        let plus = just(Token::Plus).then(unary.clone()).map(|(_, expr)| {
            DatexExpression::UnaryOperation(UnaryOperator::Plus, Box::new(expr))
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

        choice((minus, plus, logical_not, atom))
    })
}
