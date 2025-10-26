use std::fmt::Display;

use crate::ast::DatexParserTrait;
use crate::ast::data::expression::ComparisonOperation;
use crate::ast::lexer::Token;
use crate::ast::utils::operation;
use crate::ast::{DatexExpression, DatexExpressionData};
use crate::global::instruction_codes::InstructionCode;
use crate::global::protocol_structures::instructions::Instruction;
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ComparisonOperator {
    Is,                 // is
    Matches,            // matches
    StructuralEqual,    // ==
    NotStructuralEqual, // !=
    Equal,              // ===
    NotEqual,           // !==
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=
}

impl Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ComparisonOperator::Is => "is",
                ComparisonOperator::Matches => "matches",
                ComparisonOperator::StructuralEqual => "==",
                ComparisonOperator::NotStructuralEqual => "!=",
                ComparisonOperator::Equal => "===",
                ComparisonOperator::NotEqual => "!==",
                ComparisonOperator::LessThan => "<",
                ComparisonOperator::GreaterThan => ">",
                ComparisonOperator::LessThanOrEqual => "<=",
                ComparisonOperator::GreaterThanOrEqual => ">=",
            }
        )
    }
}

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

impl From<&ComparisonOperator> for InstructionCode {
    fn from(op: &ComparisonOperator) -> Self {
        match op {
            ComparisonOperator::StructuralEqual => {
                InstructionCode::STRUCTURAL_EQUAL
            }
            ComparisonOperator::NotStructuralEqual => {
                InstructionCode::NOT_STRUCTURAL_EQUAL
            }
            ComparisonOperator::Equal => InstructionCode::EQUAL,
            ComparisonOperator::NotEqual => InstructionCode::NOT_EQUAL,
            ComparisonOperator::Is => InstructionCode::IS,
            ComparisonOperator::Matches => InstructionCode::MATCHES,
            operator => todo!(
                "Comparison operator {:?} not implemented for InstructionCode",
                operator
            ),
        }
    }
}

impl From<ComparisonOperator> for InstructionCode {
    fn from(op: ComparisonOperator) -> Self {
        InstructionCode::from(&op)
    }
}
impl From<&Instruction> for ComparisonOperator {
    fn from(instruction: &Instruction) -> Self {
        match instruction {
            Instruction::StructuralEqual => ComparisonOperator::StructuralEqual,
            Instruction::Equal => ComparisonOperator::Equal,
            Instruction::NotStructuralEqual => {
                ComparisonOperator::NotStructuralEqual
            }
            Instruction::NotEqual => ComparisonOperator::NotEqual,
            Instruction::Is => ComparisonOperator::Is,
            Instruction::Matches => ComparisonOperator::Matches,
            _ => {
                todo!(
                    "Comparison operator for instruction {:?} not implemented",
                    instruction
                );
            }
        }
    }
}

impl From<Instruction> for ComparisonOperator {
    fn from(instruction: Instruction) -> Self {
        ComparisonOperator::from(&instruction)
    }
}
