use super::super::instruction_codes::InstructionCode;
use core::fmt::Display;
use core::prelude::rust_2024::*;
use datex_core::global::protocol_structures::instructions::RegularInstruction;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum AssignmentOperator {
    Assign,           // =
    AddAssign,        // +=
    SubtractAssign,   // -=
    MultiplyAssign,   // *=
    DivideAssign,     // /=
    ModuloAssign,     // %=
    PowerAssign,      // ^=
    BitwiseAndAssign, // &=
    BitwiseOrAssign,  // |=
}
impl Display for AssignmentOperator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
            f,
            "{}",
            match self {
                AssignmentOperator::Assign => "=",
                AssignmentOperator::AddAssign => "+=",
                AssignmentOperator::SubtractAssign => "-=",
                AssignmentOperator::MultiplyAssign => "*=",
                AssignmentOperator::DivideAssign => "/=",
                AssignmentOperator::ModuloAssign => "%=",
                AssignmentOperator::PowerAssign => "^=",
                AssignmentOperator::BitwiseAndAssign => "&=",
                AssignmentOperator::BitwiseOrAssign => "|=",
            }
        )
    }
}

impl From<&AssignmentOperator> for InstructionCode {
    fn from(op: &AssignmentOperator) -> Self {
        match op {
            AssignmentOperator::Assign => InstructionCode::ASSIGN,
            AssignmentOperator::AddAssign => InstructionCode::ADD_ASSIGN,
            AssignmentOperator::SubtractAssign => {
                InstructionCode::SUBTRACT_ASSIGN
            }
            AssignmentOperator::MultiplyAssign => {
                InstructionCode::MULTIPLY_ASSIGN
            }
            AssignmentOperator::DivideAssign => InstructionCode::DIVIDE_ASSIGN,
            operator => core::todo!(
                "Assignment operator {:?} not implemented for InstructionCode",
                operator
            ),
        }
    }
}

impl TryFrom<InstructionCode> for AssignmentOperator {
    type Error = ();
    fn try_from(code: InstructionCode) -> Result<Self, Self::Error> {
        Ok(match code {
            InstructionCode::ASSIGN => AssignmentOperator::Assign,
            InstructionCode::ADD_ASSIGN => AssignmentOperator::AddAssign,
            InstructionCode::SUBTRACT_ASSIGN => {
                AssignmentOperator::SubtractAssign
            }
            InstructionCode::MULTIPLY_ASSIGN => {
                AssignmentOperator::MultiplyAssign
            }
            InstructionCode::DIVIDE_ASSIGN => AssignmentOperator::DivideAssign,
            _ => return Err(()),
        })
    }
}

impl From<RegularInstruction> for AssignmentOperator {
    fn from(instruction: RegularInstruction) -> Self {
        match instruction {
            RegularInstruction::AddAssign(_) => AssignmentOperator::AddAssign,
            RegularInstruction::SubtractAssign(_) => {
                AssignmentOperator::SubtractAssign
            }
            RegularInstruction::MultiplyAssign(_) => {
                AssignmentOperator::MultiplyAssign
            }
            RegularInstruction::DivideAssign(_) => {
                AssignmentOperator::DivideAssign
            }
            _ => core::todo!(
                "Assignment operator for instruction {:?} not implemented",
                instruction
            ),
        }
    }
}
