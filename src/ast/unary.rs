use crate::ast::lexer::Token;
use crate::ast::unary_operation::{
    ArithmeticUnaryOperator, LogicalUnaryOperator, UnaryOperator,
};
use crate::ast::{DatexExpression, DatexParserTrait};
use chumsky::prelude::*;

pub fn unary<'a>(atom: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    recursive(|unary| {
        // unary minus
        let minus = just(Token::Minus).then(unary.clone()).map(|(_, expr)| {
            DatexExpression::UnaryOperation(
                UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus),
                Box::new(expr),
            )
        });
        // unary plus
        let plus = just(Token::Plus).then(unary.clone()).map(|(_, expr)| {
            DatexExpression::UnaryOperation(
                UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Plus),
                Box::new(expr),
            )
        });

        // logical NOT
        let logical_not =
            just(Token::Exclamation)
                .then(unary.clone())
                .map(|(_, expr)| {
                    DatexExpression::UnaryOperation(
                        UnaryOperator::Logical(LogicalUnaryOperator::Not),
                        Box::new(expr),
                    )
                });

        choice((minus, plus, logical_not, atom))
    })
}
