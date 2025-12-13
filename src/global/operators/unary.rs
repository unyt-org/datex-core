use crate::global::instruction_codes::InstructionCode;
use core::fmt::{Display, Formatter};
use core::prelude::rust_2024::*;
use crate::global::protocol_structures::instructions::RegularInstruction;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum UnaryOperator {
    Reference(ReferenceUnaryOperator),
    Arithmetic(ArithmeticUnaryOperator),
    Bitwise(BitwiseUnaryOperator),
    Logical(LogicalUnaryOperator),
}

impl From<&UnaryOperator> for InstructionCode {
    fn from(op: &UnaryOperator) -> Self {
        match op {
            UnaryOperator::Arithmetic(op) => InstructionCode::from(op),
            UnaryOperator::Reference(op) => InstructionCode::from(op),
            UnaryOperator::Logical(op) => InstructionCode::from(op),
            UnaryOperator::Bitwise(op) => InstructionCode::from(op),
        }
    }
}

impl From<&RegularInstruction> for UnaryOperator {
    fn from(instruction: &RegularInstruction) -> Self {
        match instruction {
            RegularInstruction::UnaryPlus => {
                UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Plus)
            }
            RegularInstruction::UnaryMinus => {
                UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus)
            }
            RegularInstruction::BitwiseNot => {
                UnaryOperator::Bitwise(BitwiseUnaryOperator::Not)
            }
            _ => {
                core::todo!(
                    "Unary operator for instruction {:?} not implemented",
                    instruction
                );
            }
        }
    }
}

impl From<RegularInstruction> for UnaryOperator {
    fn from(instruction: RegularInstruction) -> Self {
        UnaryOperator::from(&instruction)
    }
}


impl Display for UnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            UnaryOperator::Reference(op) => core::write!(f, "{}", op),
            UnaryOperator::Arithmetic(op) => core::write!(f, "{}", op),
            UnaryOperator::Bitwise(op) => core::write!(f, "{}", op),
            UnaryOperator::Logical(op) => core::write!(f, "{}", op),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ReferenceUnaryOperator {
    CreateRef,    // &
    CreateRefMut, // &mut
    Deref,        // *
}

impl From<&ReferenceUnaryOperator> for InstructionCode {
    fn from(op: &ReferenceUnaryOperator) -> Self {
        match op {
            ReferenceUnaryOperator::CreateRef => InstructionCode::CREATE_REF,
            ReferenceUnaryOperator::CreateRefMut => {
                InstructionCode::CREATE_REF_MUT
            }
            ReferenceUnaryOperator::Deref => InstructionCode::DEREF,
        }
    }
}

impl Display for ReferenceUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            ReferenceUnaryOperator::CreateRef => core::write!(f, "&"),
            ReferenceUnaryOperator::CreateRefMut => core::write!(f, "&mut"),
            ReferenceUnaryOperator::Deref => core::write!(f, "*"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ArithmeticUnaryOperator {
    Increment, // ++
    Decrement, // --
    Plus,      // +
    Minus,     // -
}

impl From<&ArithmeticUnaryOperator> for InstructionCode {
    fn from(op: &ArithmeticUnaryOperator) -> Self {
        match op {
            ArithmeticUnaryOperator::Increment => InstructionCode::INCREMENT,
            ArithmeticUnaryOperator::Decrement => InstructionCode::DECREMENT,
            ArithmeticUnaryOperator::Plus => InstructionCode::UNARY_PLUS,
            ArithmeticUnaryOperator::Minus => InstructionCode::UNARY_MINUS,
        }
    }
}

impl Display for ArithmeticUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            ArithmeticUnaryOperator::Increment => core::write!(f, "++"),
            ArithmeticUnaryOperator::Decrement => core::write!(f, "--"),
            ArithmeticUnaryOperator::Plus => core::write!(f, "+"),
            ArithmeticUnaryOperator::Minus => core::write!(f, "-"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BitwiseUnaryOperator {
    Not, // ~
}

impl From<&BitwiseUnaryOperator> for InstructionCode {
    fn from(op: &BitwiseUnaryOperator) -> Self {
        match op {
            BitwiseUnaryOperator::Not => InstructionCode::BITWISE_NOT,
        }
    }
}

impl Display for BitwiseUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            BitwiseUnaryOperator::Not => core::write!(f, "~"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum LogicalUnaryOperator {
    Not, // !
}

impl From<&LogicalUnaryOperator> for InstructionCode {
    fn from(op: &LogicalUnaryOperator) -> Self {
        match op {
            LogicalUnaryOperator::Not => InstructionCode::NOT,
        }
    }
}

impl Display for LogicalUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            LogicalUnaryOperator::Not => core::write!(f, "!"),
        }
    }
}
