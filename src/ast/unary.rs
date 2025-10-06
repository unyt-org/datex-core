use crate::ast::lexer::Token;
use crate::ast::unary_operation::{
    ArithmeticUnaryOperator, LogicalUnaryOperator, UnaryOperator,
};
use crate::ast::{DatexExpression, DatexParserTrait};
use chumsky::prelude::*;
use crate::ast::utils::whitespace;

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

        let reference = just(Token::Ampersand)
            .ignore_then(
                just(Token::Mutable)
                    .or(just(Token::Final))
                    .or_not()
                    .padded_by(whitespace()),
            )
            .then(unary.clone())
            .map(|(ref_type, expr)| match ref_type {
                Some(Token::Mutable) => DatexExpression::RefMut(Box::new(expr)),
                Some(Token::Final) => DatexExpression::RefFinal(Box::new(expr)),
                None => DatexExpression::Ref(Box::new(expr)),
                _ => unreachable!(),
            });

        let deref = just(Token::Star).then(unary.clone()).map(|(_, expr)| {
            DatexExpression::Deref(
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

        choice((minus, plus, logical_not, atom, reference, deref))
    })
}
