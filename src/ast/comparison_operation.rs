use crate::ast::DatexExpression;
use crate::ast::DatexParserTrait;
use crate::ast::TokenInput;
use crate::ast::utils::operation;
use crate::compiler::lexer::Token;
use crate::global::binary_codes::InstructionCode;
use crate::global::protocol_structures::instructions::Instruction;
use chumsky::extra::Err;
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ComparisonOperator {
    Is,                 // is
    StructuralEqual,    // ==
    NotStructuralEqual, // !=
    Equal,              // ===
    NotEqual,           // !==
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=
}

fn comparison_op(
    op: ComparisonOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| DatexExpression::ComparisonOperation(op, lhs, rhs)
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
