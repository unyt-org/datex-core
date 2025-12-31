use crate::global::instruction_codes::InstructionCode;
use crate::global::protocol_structures::instructions::RegularInstruction;
use crate::stdlib::string::ToString;
use core::fmt::Display;
use core::prelude::rust_2024::*;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BinaryOperator {
    Arithmetic(ArithmeticOperator),
    Logical(LogicalOperator),
    Bitwise(BitwiseOperator),
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

#[derive(Clone, Debug, PartialEq, Copy, Eq, Hash)]
pub enum ArithmeticOperator {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
    Power,    // ^
}
impl Display for ArithmeticOperator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
            f,
            "{}",
            match self {
                LogicalOperator::And => "and",
                LogicalOperator::Or => "or",
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
                core::todo!(
                    "Bitwise operator {:?} not implemented for InstructionCode",
                    op
                )
            }
        }
    }
}

impl Display for BitwiseOperator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
            f,
            "{}",
            match self {
                BinaryOperator::Arithmetic(op) => op.to_string(),
                BinaryOperator::Logical(op) => op.to_string(),
                BinaryOperator::Bitwise(op) => op.to_string(),
            }
        )
    }
}

impl From<&BinaryOperator> for InstructionCode {
    fn from(op: &BinaryOperator) -> Self {
        match op {
            BinaryOperator::Arithmetic(op) => InstructionCode::from(op),
            BinaryOperator::Logical(op) => InstructionCode::from(op),
            BinaryOperator::Bitwise(op) => InstructionCode::from(op),
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
            _ => core::todo!(
                "#154 Binary operator for {:?} not implemented",
                code
            ),
        }
    }
}

impl From<InstructionCode> for BinaryOperator {
    fn from(code: InstructionCode) -> Self {
        BinaryOperator::from(&code)
    }
}

impl From<&RegularInstruction> for BinaryOperator {
    fn from(instruction: &RegularInstruction) -> Self {
        match instruction {
            RegularInstruction::Add => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Add)
            }
            RegularInstruction::Subtract => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Subtract)
            }
            RegularInstruction::Multiply => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Multiply)
            }
            RegularInstruction::Divide => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide)
            }
            _ => {
                core::todo!(
                    "#155 Binary operator for instruction {:?} not implemented",
                    instruction
                );
            }
        }
    }
}

impl From<RegularInstruction> for BinaryOperator {
    fn from(instruction: RegularInstruction) -> Self {
        BinaryOperator::from(&instruction)
    }
}
