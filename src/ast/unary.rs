use crate::ast::lexer::Token;
use crate::ast::unary_operation::{
    ArithmeticUnaryOperator, LogicalUnaryOperator, UnaryOperator,
};
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
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
            .map(|(ref_type, expr)| match ref_type {
                Some(Token::Mutable) => DatexExpression::RefMut(Box::new(expr)),
                Some(Token::Final) => DatexExpression::RefFinal(Box::new(expr)),
                None => DatexExpression::Ref(Box::new(expr)),
                _ => unreachable!(),
            });

        let deref = just(Token::Star)
            .then(unary.clone())
            .map(|(_, expr)| DatexExpression::Deref(Box::new(expr)));

        // apply prefix operators repeatedly (e.g. --x or !-x)
        let prefixes = prefix_op.then(unary.clone()).map(|(op, expr)| {
            DatexExpression::UnaryOperation(op, Box::new(expr))
        });

        // try prefix forms first, fall back to atom
        choice((prefixes, reference, deref, atom))
    })
}
