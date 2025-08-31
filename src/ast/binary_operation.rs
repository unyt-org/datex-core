use std::fmt::Display;
use crate::ast::DatexExpression;
use crate::ast::DatexParserTrait;
use crate::ast::utils::is_literal;
use crate::ast::utils::operation;
use crate::compiler::lexer::Token;
use crate::global::binary_codes::InstructionCode;
use crate::global::protocol_structures::instructions::Instruction;
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BinaryOperator {
    VariantAccess, // /<literal>
    Intersection,  // &
    Union,         // |
    Add,           // +
    Subtract,      // -
    Multiply,      // *
    Divide,        // /
    Modulo,        // %
    Power,         // ^
    And,           // and
    Or,            // or
    CompositeAnd,  // TODO
    CompositeOr,   // TODO
}

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BinaryOperator::VariantAccess => "/",
            BinaryOperator::Intersection => "&",
            BinaryOperator::Union => "|",
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Power => "^",
            BinaryOperator::And => "and",
            BinaryOperator::Or => "or",
            BinaryOperator::CompositeAnd => "COMPOSITE_AND",
            BinaryOperator::CompositeOr => "COMPOSITE_OR",
        };
        write!(f, "{}", s)
    }
}


fn binary_op(
    op: BinaryOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| DatexExpression::BinaryOperation(op, lhs, rhs, None)
}

fn product<'a>(chain: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    chain
        .clone()
        .foldl(
            choice((
                operation(Token::Star).to(BinaryOperator::Multiply),
                operation(Token::Slash).to(BinaryOperator::Divide),
            ))
            .then(chain)
            .repeated(),
            |lhs, (op, rhs)| {
                let effective_op = if matches!(op, BinaryOperator::Divide)
                    && is_literal(&lhs)
                    && is_literal(&rhs)
                {
                    BinaryOperator::VariantAccess
                } else {
                    op
                };

                binary_op(effective_op)(Box::new(lhs), Box::new(rhs))
            },
        )
        .boxed()
}

fn sum<'a>(product: impl DatexParserTrait<'a>) -> impl DatexParserTrait<'a> {
    product
        .clone()
        .foldl(
            choice((
                operation(Token::Plus).to(binary_op(BinaryOperator::Add)),
                operation(Token::Minus).to(binary_op(BinaryOperator::Subtract)),
            ))
            .then(product)
            .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        )
        .boxed()
}

fn intersection<'a>(
    sum: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    sum.clone()
        .foldl(
            operation(Token::Ampersand)
                .to(binary_op(BinaryOperator::Intersection))
                .then(sum.clone())
                .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        )
        .boxed()
}

fn union<'a>(
    intersection: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
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
    chain: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    union(intersection(sum(product(chain))))
}

impl From<&BinaryOperator> for InstructionCode {
    fn from(op: &BinaryOperator) -> Self {
        match op {
            BinaryOperator::Add => InstructionCode::ADD,
            BinaryOperator::Subtract => InstructionCode::SUBTRACT,
            BinaryOperator::Multiply => InstructionCode::MULTIPLY,
            BinaryOperator::Divide => InstructionCode::DIVIDE,
            BinaryOperator::Modulo => InstructionCode::MODULO,
            BinaryOperator::Power => InstructionCode::POWER,
            BinaryOperator::And => InstructionCode::AND,
            BinaryOperator::Or => InstructionCode::OR,
            BinaryOperator::Union => InstructionCode::UNION,
            operator => todo!(
                "Binary operator {:?} not implemented for InstructionCode",
                operator
            ),
        }
    }
}

impl From<BinaryOperator> for InstructionCode {
    fn from(op: BinaryOperator) -> Self {
        InstructionCode::from(&op)
    }
}

impl From<&InstructionCode> for BinaryOperator {
    fn from(code: &InstructionCode) -> Self {
        match code {
            InstructionCode::ADD => BinaryOperator::Add,
            InstructionCode::SUBTRACT => BinaryOperator::Subtract,
            InstructionCode::MULTIPLY => BinaryOperator::Multiply,
            InstructionCode::DIVIDE => BinaryOperator::Divide,
            InstructionCode::MODULO => BinaryOperator::Modulo,
            InstructionCode::POWER => BinaryOperator::Power,
            InstructionCode::AND => BinaryOperator::And,
            InstructionCode::OR => BinaryOperator::Or,
            InstructionCode::UNION => BinaryOperator::Union,
            _ => todo!("#154 Binary operator for {:?} not implemented", code),
        }
    }
}

impl From<InstructionCode> for BinaryOperator {
    fn from(code: InstructionCode) -> Self {
        BinaryOperator::from(&code)
    }
}

impl From<&Instruction> for BinaryOperator {
    fn from(instruction: &Instruction) -> Self {
        match instruction {
            Instruction::Add => BinaryOperator::Add,
            Instruction::Subtract => BinaryOperator::Subtract,
            Instruction::Multiply => BinaryOperator::Multiply,
            Instruction::Divide => BinaryOperator::Divide,
            Instruction::Union => BinaryOperator::Union,
            _ => {
                todo!(
                    "#155 Binary operator for instruction {:?} not implemented",
                    instruction
                );
            }
        }
    }
}

impl From<Instruction> for BinaryOperator {
    fn from(instruction: Instruction) -> Self {
        BinaryOperator::from(&instruction)
    }
}
