use crate::types::definition::TypeDefinition;
use binrw::{BinRead, BinWrite};
use core::prelude::rust_2024::*;
use datex_core::references::reference::ReferenceMutability;
use num_enum::TryFromPrimitive;
use strum::Display;

#[allow(non_camel_case_types)]
#[derive(
    Debug,
    Eq,
    PartialEq,
    TryFromPrimitive,
    Copy,
    Clone,
    Display,
    num_enum::IntoPrimitive,
)]
#[repr(u8)]
pub enum TypeInstructionCode {
    TYPE_REFERENCE,
    TYPE_WITH_IMPLS,
    TYPE_UNIT,
    TYPE_UNKNOWN,
    TYPE_NEVER,
    TYPE_STRUCTURAL,
    TYPE_INTERSECTION,
    TYPE_UNION,
    TYPE_FUNCTION,
    TYPE_COLLECTION,
    TYPE_TYPE,

    TYPE_LIST,

    TYPE_LITERAL_INTEGER,
    TYPE_LITERAL_TEXT,
    TYPE_STRUCT,

    // TODO #427: Do we need std_type for optimization purpose?
    // Rename to CORE_ and implement if required
    // but TYPE TYPE_TEXT is already two bytes which is not a great benefit over the three
    // bytes for the internal pointer address + GETREF (4 vs 2 bytes)
    STD_TYPE_TEXT,
    STD_TYPE_INT,
    STD_TYPE_FLOAT,
    STD_TYPE_BOOLEAN,
    STD_TYPE_NULL,
    STD_TYPE_VOID,
    STD_TYPE_BUFFER,
    STD_TYPE_CODE_BLOCK,
    STD_TYPE_QUANTITY,
    STD_TYPE_TIME,
    STD_TYPE_URL,

    STD_TYPE_ARRAY,
    STD_TYPE_OBJECT,
    STD_TYPE_SET,
    STD_TYPE_MAP,
    STD_TYPE_TUPLE,

    STD_TYPE_FUNCTION,
    STD_TYPE_STREAM,
    STD_TYPE_ANY,
    STD_TYPE_ASSERTION,
    STD_TYPE_TASK,
    STD_TYPE_ITERATOR,
}

impl From<&TypeDefinition> for TypeInstructionCode {
    fn from(value: &TypeDefinition) -> Self {
        match value {
            TypeDefinition::ImplType(_, _) => {
                TypeInstructionCode::TYPE_WITH_IMPLS
            }
            TypeDefinition::Reference(_) => TypeInstructionCode::TYPE_REFERENCE,
            TypeDefinition::Unit => TypeInstructionCode::TYPE_UNIT,
            TypeDefinition::Unknown => TypeInstructionCode::TYPE_UNKNOWN,
            TypeDefinition::Never => TypeInstructionCode::TYPE_NEVER,
            TypeDefinition::Structural(_) => {
                TypeInstructionCode::TYPE_STRUCTURAL
            }
            TypeDefinition::Intersection(_) => {
                TypeInstructionCode::TYPE_INTERSECTION
            }
            TypeDefinition::Union(_) => TypeInstructionCode::TYPE_UNION,
            TypeDefinition::Function { .. } => {
                TypeInstructionCode::TYPE_FUNCTION
            }
            TypeDefinition::Collection(_) => {
                TypeInstructionCode::TYPE_COLLECTION
            }
            TypeDefinition::Type(_) => unreachable!(), // TODO: nested types
        }
    }
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little, repr(u8))]
pub enum TypeMutabilityCode {
    MutableReference,
    ImmutableReference,
    Value,
}

impl From<&Option<ReferenceMutability>> for TypeMutabilityCode {
    fn from(value: &Option<ReferenceMutability>) -> Self {
        match value {
            Some(ReferenceMutability::Mutable) => {
                TypeMutabilityCode::MutableReference
            }
            Some(ReferenceMutability::Immutable) => {
                TypeMutabilityCode::ImmutableReference
            }
            None => TypeMutabilityCode::Value,
        }
    }
}

impl From<TypeMutabilityCode> for Option<ReferenceMutability> {
    fn from(value: TypeMutabilityCode) -> Self {
        match value {
            TypeMutabilityCode::MutableReference => {
                Some(ReferenceMutability::Mutable)
            }
            TypeMutabilityCode::ImmutableReference => {
                Some(ReferenceMutability::Immutable)
            }
            TypeMutabilityCode::Value => None,
        }
    }
}
