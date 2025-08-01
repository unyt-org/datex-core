use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::{
    decimal::utils::decimal_to_string, endpoint::Endpoint,
};
use binrw::{BinRead, BinWrite};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    Int8(Int8Data),
    Int16(Int16Data),
    Int32(Int32Data),
    Int64(Int64Data),
    Int128(Int128Data),
    UInt128(UInt128Data),
    Endpoint(Endpoint),

    DecimalF32(Float32Data),
    DecimalF64(Float64Data),
    DecimalAsInt16(FloatAsInt16Data),
    DecimalAsInt32(FloatAsInt32Data),
    Decimal(DecimalData),

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
    Subtract,
    Multiply,
    Divide,
    Is,
    StructuralEqual,
    Equal,
    NotStructuralEqual,
    NotEqual,

    CreateRef,

    AllocateSlot(SlotAddress),
    GetSlot(SlotAddress),
    DropSlot(SlotAddress),
    UpdateSlot(SlotAddress),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Int8(data) => write!(f, "INT_8 {}", data.0),
            Instruction::Int16(data) => write!(f, "INT_16 {}", data.0),
            Instruction::Int32(data) => write!(f, "INT_32 {}", data.0),
            Instruction::Int64(data) => write!(f, "INT_64 {}", data.0),
            Instruction::Int128(data) => write!(f, "INT_128 {}", data.0),
            Instruction::UInt128(data) => write!(f, "UINT_128 {}", data.0),
            Instruction::Endpoint(data) => {
                write!(f, "ENDPOINT {data}")
            }

            Instruction::DecimalAsInt16(data) => {
                write!(f, "DECIMAL_AS_INT_16 {}", data.0)
            }
            Instruction::DecimalAsInt32(data) => {
                write!(f, "DECIMAL_AS_INT_32 {}", data.0)
            }
            Instruction::DecimalF32(data) => {
                write!(f, "DECIMAL_F32 {}", decimal_to_string(data.0, false))
            }
            Instruction::DecimalF64(data) => {
                write!(f, "DECIMAL_F64 {}", decimal_to_string(data.0, false))
            }
            Instruction::Decimal(data) => {
                write!(f, "DECIMAL_BIG {}", data.0)
            }
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

            // operations
            Instruction::Add => write!(f, "ADD"),
            Instruction::Subtract => write!(f, "SUBTRACT"),
            Instruction::Multiply => write!(f, "MULTIPLY"),
            Instruction::Divide => write!(f, "DIVIDE"),

            // equality checks
            Instruction::StructuralEqual => write!(f, "STRUCTURAL_EQUAL"),
            Instruction::Equal => write!(f, "EQUAL"),
            Instruction::NotStructuralEqual => write!(f, "NOT_STRUCTURAL_EQUAL"),
            Instruction::NotEqual => write!(f, "NOT_EQUAL"),
            Instruction::Is => write!(f, "IS"),

            Instruction::AllocateSlot(address) => {
                write!(f, "ALLOCATE_SLOT {}", address.0)
            }
            Instruction::GetSlot(address) => {
                write!(f, "GET_SLOT {}", address.0)
            }
            Instruction::DropSlot(address) => {
                write!(f, "DROP_SLOT {}", address.0)
            }
            Instruction::UpdateSlot(address) => {
                write!(f, "UPDATE_SLOT {}", address.0)
            }
            Instruction::CreateRef => write!(f, "CREATE_REF"),
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
pub struct Int128Data(pub i128);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct UInt8Data(pub u8);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct UInt16Data(pub u16);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct UInt32Data(pub u32);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct UInt64Data(pub u64);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct UInt128Data(pub u128);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct Float32Data(pub f32);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct Float64Data(pub f64);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct FloatAsInt16Data(pub i16);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct FloatAsInt32Data(pub i32);

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct DecimalData(pub Decimal);

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

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct SlotAddress(pub u32);
