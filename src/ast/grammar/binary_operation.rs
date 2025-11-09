use crate::ast::DatexParserTrait;
use crate::ast::grammar::utils::operation;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::BinaryOperation;
use crate::ast::{DatexExpression, DatexExpressionData};
use crate::global::operators::BinaryOperator;
use crate::global::operators::binary::ArithmeticOperator;
use crate::global::operators::binary::BitwiseOperator;
use crate::global::operators::binary::LogicalOperator;
use chumsky::prelude::*;

/// Generic helper for left-associative infix chains
fn infix_left_chain<'a>(
    lower: impl DatexParserTrait<'a>,
    ops: Vec<(Token, BinaryOperator)>,
) -> impl DatexParserTrait<'a> {
    let base = lower.clone();

    // Build a choice of operators
    let choices = choice(
        ops.into_iter()
            .map(|(tok, op)| operation(tok).to(op))
            .collect::<Vec<_>>(),
    );

    base.clone()
        .foldl(
            choices.then(base.clone()).repeated(),
            move |lhs, (op, rhs)| binary_op(op)(Box::new(lhs), Box::new(rhs)),
        )
        .boxed()
}

fn binary_op(
    op: BinaryOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| {
        let start = lhs.span.start.min(rhs.span.start);
        let end = lhs.span.end.max(rhs.span.end);
        let combined_span = start..end;
        DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: op,
            left: lhs,
            right: rhs,
            ty: None,
        })
        .with_span(SimpleSpan::from(combined_span))
    }
}
fn product<'a>(atom: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    infix_left_chain(
        atom,
        vec![
            (Token::Star, ArithmeticOperator::Multiply.into()),
            (Token::Slash, ArithmeticOperator::Divide.into()),
        ],
    )
}
fn power<'a>(product: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    infix_left_chain(
        product,
        vec![(Token::Caret, ArithmeticOperator::Power.into())],
    )
}

fn sum<'a>(prod: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    infix_left_chain(
        prod,
        vec![
            (Token::Plus, ArithmeticOperator::Add.into()),
            (Token::Minus, ArithmeticOperator::Subtract.into()),
        ],
    )
}

fn bitwise_and<'a>(
    sum: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    infix_left_chain(sum, vec![(Token::Ampersand, BitwiseOperator::And.into())])
}

fn bitwise_or<'a>(
    bitwise_and: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    infix_left_chain(
        bitwise_and,
        vec![(Token::Pipe, BitwiseOperator::Or.into())],
    )
}

fn logical_and<'a>(
    bitwise_or: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    infix_left_chain(
        bitwise_or,
        vec![(Token::DoubleAnd, LogicalOperator::And.into())],
    )
}

fn logical_or<'a>(
    logical_and: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    infix_left_chain(
        logical_and,
        vec![(Token::DoublePipe, LogicalOperator::Or.into())],
    )
}

pub fn binary_operation<'a>(
    atom: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    logical_or(logical_and(bitwise_or(bitwise_and(sum(product(power(
        atom,
    )))))))
}
