use crate::global::operators::AssignmentOperator;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::{
    decimal::utils::decimal_to_string, endpoint::Endpoint,
};
use binrw::{BinRead, BinWrite};
use core::fmt::Display;
use core::prelude::rust_2024::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    // signed integers
    Int8(Int8Data),
    Int16(Int16Data),
    Int32(Int32Data),
    Int64(Int64Data),
    Int128(Int128Data),

    // unsigned integers
    UInt8(UInt8Data),
    UInt16(UInt16Data),
    UInt32(UInt32Data),
    UInt64(UInt64Data),
    UInt128(UInt128Data),

    // big integers
    BigInteger(IntegerData),

    Range(RangeData),

    Endpoint(Endpoint),

    DecimalF32(Float32Data),
    DecimalF64(Float64Data),
    DecimalAsInt16(FloatAsInt16Data),
    DecimalAsInt32(FloatAsInt32Data),
    Decimal(DecimalData),

    ExecutionBlock(ExecutionBlockData),
    RemoteExecution,

    ShortText(ShortTextData),
    Text(TextData),
    True,
    False,
    Null,
    ScopeStart,
    ListStart,
    MapStart,
    StructStart,
    ScopeEnd,
    KeyValueDynamic,
    KeyValueShortText(ShortTextData),
    CloseAndStore,

    // binary operator
    Add,
    Subtract,
    Multiply,
    Divide,

    // unary operator
    // TODO #432 add missing unary operators
    UnaryMinus,
    // TODO #433: Do we need this for op overloading or can we avoid?
    UnaryPlus,
    BitwiseNot,

    Apply(ApplyData),

    // comparison operator
    Is,
    Matches,
    StructuralEqual,
    Equal,
    NotStructuralEqual,
    NotEqual,

    // assignment operator
    AddAssign(SlotAddress),
    SubtractAssign(SlotAddress),
    MultiplyAssign(SlotAddress),
    DivideAssign(SlotAddress),

    CreateRef,
    CreateRefMut,

    // &ABCDE
    GetRef(RawFullPointerAddress),
    GetLocalRef(RawLocalPointerAddress),
    GetInternalRef(RawInternalPointerAddress),

    // &ABCDE := ...
    GetOrCreateRef(GetOrCreateRefData),
    // &mut ABCDE := ...
    GetOrCreateRefMut(GetOrCreateRefData),

    AllocateSlot(SlotAddress),
    GetSlot(SlotAddress),
    DropSlot(SlotAddress),
    SetSlot(SlotAddress),

    AssignToReference(AssignmentOperator),
    Deref,

    TypeInstructions(Vec<TypeInstruction>),
    TypeExpression(Vec<TypeInstruction>),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Instruction::Int8(data) => core::write!(f, "INT_8 {}", data.0),
            Instruction::Int16(data) => core::write!(f, "INT_16 {}", data.0),
            Instruction::Int32(data) => core::write!(f, "INT_32 {}", data.0),
            Instruction::Int64(data) => core::write!(f, "INT_64 {}", data.0),
            Instruction::Int128(data) => core::write!(f, "INT_128 {}", data.0),

            Instruction::UInt8(data) => core::write!(f, "UINT_8 {}", data.0),
            Instruction::UInt16(data) => core::write!(f, "UINT_16 {}", data.0),
            Instruction::UInt32(data) => core::write!(f, "UINT_32 {}", data.0),
            Instruction::UInt64(data) => core::write!(f, "UINT_64 {}", data.0),
            Instruction::UInt128(data) => {
                core::write!(f, "UINT_128 {}", data.0)
            }

            Instruction::Range(data) => {
                core::write!(f, "RANGE {} {}", data.start, data.end)
            }

            Instruction::Apply(count) => {
                core::write!(f, "APPLY {}", count.arg_count)
            }

            Instruction::BigInteger(data) => {
                core::write!(f, "BIG_INTEGER {}", data.0)
            }
            Instruction::Endpoint(data) => {
                core::write!(f, "ENDPOINT {data}")
            }

            Instruction::DecimalAsInt16(data) => {
                core::write!(f, "DECIMAL_AS_INT_16 {}", data.0)
            }
            Instruction::DecimalAsInt32(data) => {
                core::write!(f, "DECIMAL_AS_INT_32 {}", data.0)
            }
            Instruction::DecimalF32(data) => {
                core::write!(
                    f,
                    "DECIMAL_F32 {}",
                    decimal_to_string(data.0, false)
                )
            }
            Instruction::DecimalF64(data) => {
                core::write!(
                    f,
                    "DECIMAL_F64 {}",
                    decimal_to_string(data.0, false)
                )
            }
            Instruction::Decimal(data) => {
                core::write!(f, "DECIMAL_BIG {}", data.0)
            }
            Instruction::ShortText(data) => {
                core::write!(f, "SHORT_TEXT {}", data.0)
            }
            Instruction::Text(data) => core::write!(f, "TEXT {}", data.0),
            Instruction::True => core::write!(f, "TRUE"),
            Instruction::False => core::write!(f, "FALSE"),
            Instruction::Null => core::write!(f, "NULL"),
            Instruction::ScopeStart => core::write!(f, "SCOPE_START"),
            Instruction::ListStart => core::write!(f, "LIST_START"),
            Instruction::MapStart => core::write!(f, "MAP_START"),
            Instruction::StructStart => core::write!(f, "STRUCT_START"),
            Instruction::ScopeEnd => core::write!(f, "SCOPE_END"),
            Instruction::KeyValueDynamic => {
                core::write!(f, "KEY_VALUE_DYNAMIC")
            }
            Instruction::KeyValueShortText(data) => {
                core::write!(f, "KEY_VALUE_SHORT_TEXT {}", data.0)
            }
            Instruction::CloseAndStore => core::write!(f, "CLOSE_AND_STORE"),

            // operations
            Instruction::Add => core::write!(f, "ADD"),
            Instruction::Subtract => core::write!(f, "SUBTRACT"),
            Instruction::Multiply => core::write!(f, "MULTIPLY"),
            Instruction::Divide => core::write!(f, "DIVIDE"),

            // equality checks
            Instruction::StructuralEqual => core::write!(f, "STRUCTURAL_EQUAL"),
            Instruction::Equal => core::write!(f, "EQUAL"),
            Instruction::NotStructuralEqual => {
                core::write!(f, "NOT_STRUCTURAL_EQUAL")
            }
            Instruction::NotEqual => core::write!(f, "NOT_EQUAL"),
            Instruction::Is => core::write!(f, "IS"),
            Instruction::Matches => core::write!(f, "MATCHES"),

            Instruction::AllocateSlot(address) => {
                core::write!(f, "ALLOCATE_SLOT {}", address.0)
            }
            Instruction::GetSlot(address) => {
                core::write!(f, "GET_SLOT {}", address.0)
            }
            Instruction::DropSlot(address) => {
                core::write!(f, "DROP_SLOT {}", address.0)
            }
            Instruction::SetSlot(address) => {
                core::write!(f, "SET_SLOT {}", address.0)
            }
            Instruction::AssignToReference(operator) => {
                core::write!(f, "ASSIGN_REFERENCE ({})", operator)
            }
            Instruction::Deref => core::write!(f, "DEREF"),
            Instruction::GetRef(address) => {
                core::write!(
                    f,
                    "GET_REF [{}:{}]",
                    address.endpoint,
                    hex::encode(address.id)
                )
            }
            Instruction::GetLocalRef(address) => {
                core::write!(
                    f,
                    "GET_LOCAL_REF [origin_id: {}]",
                    hex::encode(address.id)
                )
            }
            Instruction::GetInternalRef(address) => {
                core::write!(
                    f,
                    "GET_INTERNAL_REF [internal_id: {}]",
                    hex::encode(address.id)
                )
            }
            Instruction::CreateRef => core::write!(f, "CREATE_REF"),
            Instruction::CreateRefMut => core::write!(f, "CREATE_REF_MUT"),
            Instruction::GetOrCreateRef(data) => {
                core::write!(
                    f,
                    "GET_OR_CREATE_REF [{}:{}, block_size: {}]",
                    data.address.endpoint,
                    hex::encode(data.address.id),
                    data.create_block_size
                )
            }
            Instruction::GetOrCreateRefMut(data) => {
                core::write!(
                    f,
                    "GET_OR_CREATE_REF_MUT [{}:{}, block_size: {}]",
                    data.address.endpoint,
                    hex::encode(data.address.id),
                    data.create_block_size
                )
            }
            Instruction::ExecutionBlock(block) => {
                core::write!(
                    f,
                    "EXECUTION_BLOCK (length: {}, injected_slot_count: {})",
                    block.length,
                    block.injected_slot_count
                )
            }
            Instruction::RemoteExecution => core::write!(f, "REMOTE_EXECUTION"),
            Instruction::AddAssign(address) => {
                core::write!(f, "ADD_ASSIGN {}", address.0)
            }
            Instruction::SubtractAssign(address) => {
                core::write!(f, "SUBTRACT_ASSIGN {}", address.0)
            }
            Instruction::MultiplyAssign(address) => {
                core::write!(f, "MULTIPLY_ASSIGN {}", address.0)
            }
            Instruction::DivideAssign(address) => {
                core::write!(f, "DIVIDE_ASSIGN {}", address.0)
            }
            Instruction::TypeInstructions(instr) => {
                let instr_strings: Vec<String> =
                    instr.iter().map(|i| i.to_string()).collect();
                core::write!(
                    f,
                    "TYPE_INSTRUCTIONS [{}]",
                    instr_strings.join(", ")
                )
            }
            Instruction::TypeExpression(instr) => {
                let instr_strings: Vec<String> =
                    instr.iter().map(|i| i.to_string()).collect();
                core::write!(
                    f,
                    "TYPE_EXPRESSION [{}]",
                    instr_strings.join(", ")
                )
            }
            Instruction::UnaryMinus => core::write!(f, "-"),
            Instruction::UnaryPlus => core::write!(f, "+"),
            Instruction::BitwiseNot => core::write!(f, "BITWISE_NOT"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeInstruction {
    LiteralText(TextData),
    LiteralInteger(IntegerData),
    ListStart,
    ScopeEnd,
}

impl Display for TypeInstruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TypeInstruction::LiteralText(data) => {
                core::write!(f, "LITERAL_TEXT {}", data.0)
            }
            TypeInstruction::LiteralInteger(data) => {
                core::write!(f, "LITERAL_INTEGER {}", data.0)
            }
            TypeInstruction::ListStart => core::write!(f, "LIST_START"),
            TypeInstruction::ScopeEnd => core::write!(f, "SCOPE_END"),
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
pub struct IntegerData(pub Integer);

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

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct RawFullPointerAddress {
    pub endpoint: Endpoint,
    pub id: [u8; 5],
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct RawLocalPointerAddress {
    pub id: [u8; 5],
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct RawInternalPointerAddress {
    pub id: [u8; 3],
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct GetOrCreateRefData {
    pub address: RawFullPointerAddress,
    pub create_block_size: u64,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct ExecutionBlockData {
    pub length: u32,
    pub injected_slot_count: u32,
    #[br(count = injected_slot_count)]
    pub injected_slots: Vec<u32>,
    #[br(count = length)]
    pub body: Vec<u8>,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct ApplyData {
    pub arg_count: u16,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct RangeData {
    pub ignored: u8,
    pub start: u8,
    pub other_ignored: u8,
    pub end: u8,
}
