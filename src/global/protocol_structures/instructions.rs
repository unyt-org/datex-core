use binrw::{BinRead, BinWrite};
use std::fmt::Display;
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    Int8(Int8Data),
    Int16(Int16Data),
    Int32(Int32Data),
    Int64(Int64Data),
    Float64(Float64Data),
    ShortText(ShortTextData),
    Text(TextData),
    True,
    False,
    Null,
    ScopeStart,
    ArrayStart,
    ObjectStart,
    TupleStart,
    ScopeEnd,
    KeyValueDynamic,
    KeyValueShortText(ShortTextData),
    CloseAndStore,
    Add,
    Multiply,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Int8(data) => write!(f, "INT_8 {}", data.0),
            Instruction::Int16(data) => write!(f, "INT_16 {}", data.0),
            Instruction::Int32(data) => write!(f, "INT_32 {}", data.0),
            Instruction::Int64(data) => write!(f, "INT_64 {}", data.0),
            Instruction::Float64(data) => write!(f, "FLOAT_64 {}", data.0),
            Instruction::ShortText(data) => write!(f, "SHORT_TEXT {}", data.0),
            Instruction::Text(data) => write!(f, "TEXT {}", data.0),
            Instruction::True => write!(f, "TRUE"),
            Instruction::False => write!(f, "FALSE"),
            Instruction::Null => write!(f, "NULL"),
            Instruction::ScopeStart => write!(f, "SCOPE_START"),
            Instruction::ArrayStart => write!(f, "ARRAY_START"),
            Instruction::ObjectStart => write!(f, "OBJECT_START"),
            Instruction::TupleStart => write!(f, "TUPLE_START"),
            Instruction::ScopeEnd => write!(f, "SCOPE_END"),
            Instruction::KeyValueDynamic => write!(f, "KEY_VALUE_DYNAMIC"),
            Instruction::KeyValueShortText(data) => {
                write!(f, "KEY_VALUE_SHORT_TEXT {}", data.0)
            }
            Instruction::CloseAndStore => write!(f, "CLOSE_AND_STORE"),
            Instruction::Add => write!(f, "ADD"),
            Instruction::Multiply => write!(f, "MULTIPLY"),
        }
    }
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct Int8Data(pub i8);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct Int16Data(pub i16);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct Int32Data(pub i32);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct Int64Data(pub i64);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct Float64Data(pub f64);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct ShortTextDataRaw {
    pub length: u8,
    #[br(count = length)]
    pub text: Vec<u8>,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct TextDataRaw {
    pub length: u32,
    #[br(count = length)]
    pub text: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShortTextData(pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct TextData(pub String);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct InstructionCloseAndStore {
    pub instruction: Int8Data,
}
