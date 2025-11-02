use core::prelude::rust_2024::*;
use core::result::Result;
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
pub enum TypeSpaceInstructionCode {
    TYPE_REFERENCE,

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
