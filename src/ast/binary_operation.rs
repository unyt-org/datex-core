use crate::ast::TokenInput;
use crate::ast::utils::operation;
use crate::compiler::ast_parser::{BinaryOperator, DatexExpression};
use crate::compiler::lexer::Token;
use chumsky::extra::{Err, Full};
use chumsky::prelude::*;

fn binary_op(
    op: BinaryOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| DatexExpression::BinaryOperation(op, lhs, rhs)
}

fn product<'a>(
    chain: impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a,
) -> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a {
    chain.clone().foldl(
        choice((
            operation(Token::Star).to(binary_op(BinaryOperator::Multiply)),
            operation(Token::Slash).to(binary_op(BinaryOperator::Divide)),
        ))
        .then(chain)
        .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    )
}

fn sum<'a>(
    product: impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>>
    + Clone
    + 'a,
) -> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a {
    product.clone().foldl(
        choice((
            operation(Token::Plus).to(binary_op(BinaryOperator::Add)),
            operation(Token::Minus).to(binary_op(BinaryOperator::Subtract)),
        ))
        .then(product)
        .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    )
}

fn intersection<'a>(
    sum: impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a,
) -> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a {
    sum.clone()
        .foldl(
            operation(Token::Ampersand)
                .to(binary_op(BinaryOperator::Intersection))
                .then(sum)
                .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        )
        .boxed()
}

fn union<'a>(
    intersection: impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>>
    + Clone
    + 'a,
) -> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a {
    intersection
        .clone()
        .foldl(
            operation(Token::Pipe)
                .to(binary_op(BinaryOperator::Union))
                .then(intersection.clone())
                .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        )
        .boxed()
}

pub fn binary_operation<'a>(
    chain: impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a,
) -> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>> + Clone + 'a {
    union(intersection(sum(product(chain))))
}
