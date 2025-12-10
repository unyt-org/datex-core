use core::prelude::rust_2024::*;
use num_enum::TryFromPrimitive;
use strum::Display;
use datex_core::references::reference::ReferenceMutability;
use crate::types::definition::TypeDefinition;

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
pub enum TypeSpaceInstructionCode {
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
    
    TYPE_LIST_START,
    TYPE_SCOPE_END,

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


impl From<&TypeDefinition> for TypeSpaceInstructionCode {
    fn from(value: &TypeDefinition) -> Self {
        match value {
            TypeDefinition::ImplType(_, _) => TypeSpaceInstructionCode::TYPE_WITH_IMPLS,
            TypeDefinition::Reference(_) => TypeSpaceInstructionCode::TYPE_REFERENCE,
            TypeDefinition::Unit => TypeSpaceInstructionCode::TYPE_UNIT,
            TypeDefinition::Unknown => TypeSpaceInstructionCode::TYPE_UNKNOWN,
            TypeDefinition::Never => TypeSpaceInstructionCode::TYPE_NEVER,
            TypeDefinition::Structural(_) => TypeSpaceInstructionCode::TYPE_STRUCTURAL,
            TypeDefinition::Intersection(_) => TypeSpaceInstructionCode::TYPE_INTERSECTION,
            TypeDefinition::Union(_) => TypeSpaceInstructionCode::TYPE_UNION,
            TypeDefinition::Function {..} => TypeSpaceInstructionCode::TYPE_FUNCTION,
            TypeDefinition::Collection(_) => TypeSpaceInstructionCode::TYPE_COLLECTION,
            TypeDefinition::Type(_) => unreachable!(), // TODO: nested types
        }
    }
}


pub enum TypeMutabilityCode {
    MutableReference,
    ImmutableReference,
    Value
}

impl From<&Option<ReferenceMutability>> for TypeMutabilityCode {
    fn from(value: &Option<ReferenceMutability>) -> Self {
        match value {
            Some(ReferenceMutability::Mutable) => TypeMutabilityCode::MutableReference,
            Some(ReferenceMutability::Immutable) => TypeMutabilityCode::ImmutableReference,
            None => TypeMutabilityCode::Value,
        }
    }
}