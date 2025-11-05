use crate::ast::DatexParserTrait;
use crate::ast::grammar::utils::operation;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::ComparisonOperation;
use crate::ast::structs::operator::ComparisonOperator;
use crate::ast::{DatexExpression, DatexExpressionData};
use chumsky::prelude::*;

fn comparison_op(
    op: ComparisonOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| {
        let start = lhs.span.start.min(rhs.span.start);
        let end = lhs.span.end.max(rhs.span.end);
        let combined_span = start..end;
        DatexExpressionData::ComparisonOperation(ComparisonOperation {
            operator: op,
            left: lhs,
            right: rhs,
        })
        .with_span(SimpleSpan::from(combined_span))
    }
}

pub fn comparison_operation<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    union
        .clone()
        .foldl(
            choice((
                operation(Token::StructuralEqual)
                    .to(comparison_op(ComparisonOperator::StructuralEqual)),
                operation(Token::Equal)
                    .to(comparison_op(ComparisonOperator::Equal)),
                operation(Token::NotStructuralEqual)
                    .to(comparison_op(ComparisonOperator::NotStructuralEqual)),
                operation(Token::NotEqual)
                    .to(comparison_op(ComparisonOperator::NotEqual)),
                operation(Token::Is).to(comparison_op(ComparisonOperator::Is)),
                operation(Token::Matches)
                    .to(comparison_op(ComparisonOperator::Matches)),
            ))
            .then(union.clone())
            .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        )
        .boxed()
}
