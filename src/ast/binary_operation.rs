use crate::ast::DatexParserTrait;
use crate::ast::lexer::Token;
use crate::ast::utils::is_identifier;
use crate::ast::utils::operation;
use crate::ast::{DatexExpression, DatexExpressionData};
use crate::global::instruction_codes::InstructionCode;
use crate::global::protocol_structures::instructions::Instruction;
use chumsky::prelude::*;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BinaryOperator {
    Arithmetic(ArithmeticOperator),
    Logical(LogicalOperator),
    Bitwise(BitwiseOperator),
    VariantAccess,
}
impl From<ArithmeticOperator> for BinaryOperator {
    fn from(op: ArithmeticOperator) -> Self {
        BinaryOperator::Arithmetic(op)
    }
}
impl From<LogicalOperator> for BinaryOperator {
    fn from(op: LogicalOperator) -> Self {
        BinaryOperator::Logical(op)
    }
}
impl From<BitwiseOperator> for BinaryOperator {
    fn from(op: BitwiseOperator) -> Self {
        BinaryOperator::Bitwise(op)
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ArithmeticOperator {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
    Power,    // ^
}
impl Display for ArithmeticOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ArithmeticOperator::Add => "+",
                ArithmeticOperator::Subtract => "-",
                ArithmeticOperator::Multiply => "*",
                ArithmeticOperator::Divide => "/",
                ArithmeticOperator::Modulo => "%",
                ArithmeticOperator::Power => "^",
            }
        )
    }
}
impl From<&ArithmeticOperator> for InstructionCode {
    fn from(op: &ArithmeticOperator) -> Self {
        match op {
            ArithmeticOperator::Add => InstructionCode::ADD,
            ArithmeticOperator::Subtract => InstructionCode::SUBTRACT,
            ArithmeticOperator::Multiply => InstructionCode::MULTIPLY,
            ArithmeticOperator::Divide => InstructionCode::DIVIDE,
            ArithmeticOperator::Modulo => InstructionCode::MODULO,
            ArithmeticOperator::Power => InstructionCode::POWER,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum LogicalOperator {
    And, // and
    Or,  // or
}

impl From<&LogicalOperator> for InstructionCode {
    fn from(op: &LogicalOperator) -> Self {
        match op {
            LogicalOperator::And => InstructionCode::AND,
            LogicalOperator::Or => InstructionCode::OR,
        }
    }
}

impl Display for LogicalOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LogicalOperator::And => "&&",
                LogicalOperator::Or => "||",
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BitwiseOperator {
    And, // &
    Or,  // |
    Xor, // ^
    Not, // ~
}

impl From<&BitwiseOperator> for InstructionCode {
    fn from(op: &BitwiseOperator) -> Self {
        match op {
            BitwiseOperator::And => InstructionCode::AND,
            BitwiseOperator::Or => InstructionCode::OR,
            BitwiseOperator::Not => InstructionCode::NOT,
            _ => {
                todo!(
                    "Bitwise operator {:?} not implemented for InstructionCode",
                    op
                )
            }
        }
    }
}

impl Display for BitwiseOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BitwiseOperator::And => "&",
                BitwiseOperator::Or => "|",
                BitwiseOperator::Xor => "^",
                BitwiseOperator::Not => "~",
            }
        )
    }
}

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryOperator::Arithmetic(op) => op.to_string(),
                BinaryOperator::Logical(op) => op.to_string(),
                BinaryOperator::Bitwise(op) => op.to_string(),
                BinaryOperator::VariantAccess => "/".to_string(),
            }
        )
    }
}

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
            move |lhs, (op, rhs)| {
                // Special handling for division between identifiers
                let effective_op = match op {
                    BinaryOperator::Arithmetic(ArithmeticOperator::Divide) => {
                        if is_identifier(&lhs) && is_identifier(&rhs) {
                            BinaryOperator::VariantAccess
                        } else {
                            op
                        }
                    }
                    _ => op,
                };

                binary_op(effective_op)(Box::new(lhs), Box::new(rhs))
            },
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
        DatexExpressionData::BinaryOperation(op, lhs, rhs, None)
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

impl From<&BinaryOperator> for InstructionCode {
    fn from(op: &BinaryOperator) -> Self {
        match op {
            BinaryOperator::Arithmetic(op) => InstructionCode::from(op),
            BinaryOperator::Logical(op) => InstructionCode::from(op),
            BinaryOperator::Bitwise(op) => InstructionCode::from(op),
            BinaryOperator::VariantAccess => {
                todo!("#355 VariantAccess not implemented for InstructionCode")
            }
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
            InstructionCode::ADD => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Add)
            }
            InstructionCode::SUBTRACT => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Subtract)
            }
            InstructionCode::MULTIPLY => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Multiply)
            }
            InstructionCode::DIVIDE => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide)
            }
            InstructionCode::MODULO => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Modulo)
            }
            InstructionCode::POWER => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Power)
            }
            InstructionCode::AND => {
                BinaryOperator::Logical(LogicalOperator::And)
            }
            InstructionCode::OR => BinaryOperator::Logical(LogicalOperator::Or),
            InstructionCode::UNION => {
                BinaryOperator::Bitwise(BitwiseOperator::And)
            }
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
            Instruction::Add => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Add)
            }
            Instruction::Subtract => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Subtract)
            }
            Instruction::Multiply => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Multiply)
            }
            Instruction::Divide => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide)
            }
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
