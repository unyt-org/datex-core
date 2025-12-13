use crate::global::instruction_codes::InstructionCode;
use crate::global::protocol_structures::instructions::RegularInstruction;
use core::fmt::Display;
use core::prelude::rust_2024::*;

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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
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
            operator => core::todo!(
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
impl From<&RegularInstruction> for ComparisonOperator {
    fn from(instruction: &RegularInstruction) -> Self {
        match instruction {
            RegularInstruction::StructuralEqual => ComparisonOperator::StructuralEqual,
            RegularInstruction::Equal => ComparisonOperator::Equal,
            RegularInstruction::NotStructuralEqual => {
                ComparisonOperator::NotStructuralEqual
            }
            RegularInstruction::NotEqual => ComparisonOperator::NotEqual,
            RegularInstruction::Is => ComparisonOperator::Is,
            RegularInstruction::Matches => ComparisonOperator::Matches,
            _ => {
                core::todo!(
                    "Comparison operator for instruction {:?} not implemented",
                    instruction
                );
            }
        }
    }
}

impl From<RegularInstruction> for ComparisonOperator {
    fn from(instruction: RegularInstruction) -> Self {
        ComparisonOperator::from(&instruction)
    }
}
