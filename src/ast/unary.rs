use crate::ast::unary_operation::UnaryOperator;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::compiler::lexer::Token;
use chumsky::extra::Err;
use chumsky::prelude::*;

// pub fn unary<'a>(atom: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
//     recursive(|unary| {
//         // & or &mut prefix
//         just(Token::Ampersand)
//             .ignore_then(just(Token::Mutable).or_not().padded_by(whitespace()))
//             .then(unary)
//             .map(|(mut_kw, expr)| {
//                 if mut_kw.is_some() {
//                     DatexExpression::RefMut(Box::new(expr))
//                 } else {
//                     DatexExpression::Ref(Box::new(expr))
//                 }
//             })
//             // could also add unary minus, not, etc. here later
//             .or(atom)
//     })
// }

pub fn unary<'a>(atom: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    recursive(|unary| {
        // & or &mut reference
        let reference = just(Token::Ampersand)
            .ignore_then(just(Token::Mutable).or_not().padded_by(whitespace()))
            .then(unary.clone())
            .map(|(mut_kw, expr)| {
                if mut_kw.is_some() {
                    DatexExpression::RefMut(Box::new(expr))
                } else {
                    DatexExpression::Ref(Box::new(expr))
                }
            });

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

        choice((reference, negation, logical_not, atom))
    })
}
