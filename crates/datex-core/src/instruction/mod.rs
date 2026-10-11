//! This module contains all instruction related structs and enums, such as [InstructionCode](instruction_codes::InstructionCode), [InstructionData](instruction_data), [RegularInstruction](regular_instruction::RegularInstruction), and [TypeInstruction](type_instruction).
pub mod instruction_codes;
pub mod instruction_data;
pub mod regular_instruction;
pub mod type_instruction;
pub mod type_instruction_codes;

use crate::{
    instruction::{
        regular_instruction::RegularInstruction,
        type_instruction::TypeInstruction,
    },
    prelude::*,
};
use binrw::{
    BinResult, BinWrite, Endian,
    io::{Seek, Write},
    meta::{EndianKind, WriteEndian},
};
use core::{fmt::Display, prelude::rust_2024::*};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "disassembler", derive(Serialize))]
#[cfg_attr(feature = "disassembler", serde(untagged))]
pub enum Instruction {
    // regular instruction
    Regular(RegularInstruction),
    // Type instruction that yields a type
    Type(TypeInstruction),
}

impl Instruction {
    pub fn get_next_expected_instructions(&self) -> NextExpectedInstructions {
        match self {
            Instruction::Regular(instr) => {
                instr.get_next_expected_instructions()
            }
            Instruction::Type(instr) => instr.get_next_expected_instructions(),
        }
    }

    pub fn metadata_string(&self) -> Option<String> {
        match self {
            Instruction::Regular(instr) => instr.metadata_string(),
            Instruction::Type(instr) => instr.metadata_string(),
        }
    }
}

impl BinWrite for Instruction {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<()> {
        match self {
            Instruction::Regular(instr) => {
                instr.write_options(writer, endian, ())
            }
            Instruction::Type(instr) => instr.write_options(writer, endian, ()),
        }
    }
}

impl WriteEndian for Instruction {
    const ENDIAN: EndianKind = EndianKind::Endian(Endian::Little);
}

impl Display for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Instruction::Regular(instr) => {
                write!(f, "{}", instr)
            }
            Instruction::Type(instr) => {
                write!(f, "TYPE_INSTRUCTION {}", instr)
            }
        }
    }
}

impl From<RegularInstruction> for Instruction {
    fn from(instruction: RegularInstruction) -> Self {
        Instruction::Regular(instruction)
    }
}

impl From<TypeInstruction> for Instruction {
    fn from(instruction: TypeInstruction) -> Self {
        Instruction::Type(instruction)
    }
}

pub enum NextExpectedInstructions {
    None,
    Regular(u32),
    Type(u32),
    UnboundedStart,
    UnboundedEnd,
    RegularAndType(u32, u32),
}

pub enum CountOrUnbounded {
    Count(u32),
    UnboundedStart,
    UnboundedEnd,
}

impl NextExpectedInstructions {
    pub fn total_count(&self) -> Option<CountOrUnbounded> {
        match self {
            NextExpectedInstructions::None => None,
            NextExpectedInstructions::Regular(count) => {
                if count == &0 {
                    None
                } else {
                    Some(CountOrUnbounded::Count(*count))
                }
            }
            NextExpectedInstructions::Type(count) => {
                if count == &0 {
                    None
                } else {
                    Some(CountOrUnbounded::Count(*count))
                }
            }
            NextExpectedInstructions::UnboundedStart => {
                Some(CountOrUnbounded::UnboundedStart)
            }
            NextExpectedInstructions::UnboundedEnd => {
                Some(CountOrUnbounded::UnboundedEnd)
            }
            NextExpectedInstructions::RegularAndType(
                regular_count,
                type_count,
            ) => {
                let total_count = regular_count + type_count;
                if total_count == 0 {
                    None
                } else {
                    Some(CountOrUnbounded::Count(total_count))
                }
            }
        }
    }
}

#[derive(Default, Clone, Debug, Copy, PartialEq, Eq)]
pub enum NestedInstructionResolutionStrategy {
    /// Does not resolve nested instructions scopes (e.g. for remote execution)
    #[default]
    None,
    /// Recursively resolves nested instructions scopes (e.g. for remote execution) as an instructions vector
    ResolveNestedScopesFlat,
    /// Recursively resolves nested instructions scopes (e.g. for remote execution) as an instructions tree structure
    ResolveNestedScopesTree,
}
