use crate::ast::grammar::utils::whitespace;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::UnaryOperation;
use crate::ast::structs::operator::{
    ArithmeticUnaryOperator, LogicalUnaryOperator, UnaryOperator,
};
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;

pub fn unary<'a>(atom: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    recursive(|unary| {
        // prefix minus/plus/not
        let prefix_op = choice((
            just(Token::Minus)
                .to(UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus)),
            just(Token::Plus)
                .to(UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Plus)),
            just(Token::Exclamation)
                .to(UnaryOperator::Logical(LogicalUnaryOperator::Not)),
        ));

        // references and deref as prefix forms that consume the next unary
        let reference = just(Token::Ampersand)
            .ignore_then(
                just(Token::Mutable)
                    .or(just(Token::Final))
                    .or_not()
                    .padded_by(whitespace()),
            )
            .then(unary.clone())
            .map_with(|(ref_type, expr), e| {
                match ref_type {
                    Some(Token::Mutable) => {
                        DatexExpressionData::CreateRefMut(Box::new(expr))
                    }
                    Some(Token::Final) => {
                        DatexExpressionData::CreateRefFinal(Box::new(expr))
                    }
                    None => DatexExpressionData::CreateRef(Box::new(expr)),
                    _ => unreachable!(),
                }
                .with_span(e.span())
            });

        let deref =
            just(Token::Star)
                .then(unary.clone())
                .map_with(|(_, expr), e| {
                    DatexExpressionData::Deref(Box::new(expr))
                        .with_span(e.span())
                });

        // apply prefix operators repeatedly (e.g. --x or !-x)
        let prefixes =
            prefix_op.then(unary.clone()).map_with(|(op, expr), e| {
                DatexExpressionData::UnaryOperation(UnaryOperation {
                    operator: op,
                    expression: Box::new(expr),
                })
                .with_span(e.span())
            });

        // try prefix forms first, fall back to atom
        choice((prefixes, reference, deref, atom))
    })
}
