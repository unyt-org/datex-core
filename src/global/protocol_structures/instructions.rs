use crate::ast::assignment_operation::AssignmentOperator;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::{
    decimal::utils::decimal_to_string, endpoint::Endpoint,
};
use binrw::{BinRead, BinWrite};
use std::fmt::Display;

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
    Union,

    // unary operator
    // TODO add missing unary operators
    UnaryMinus,
    // TODO: Do we need this for op overloading or can we avoid?
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
    CreateRefFinal,

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Int8(data) => write!(f, "INT_8 {}", data.0),
            Instruction::Int16(data) => write!(f, "INT_16 {}", data.0),
            Instruction::Int32(data) => write!(f, "INT_32 {}", data.0),
            Instruction::Int64(data) => write!(f, "INT_64 {}", data.0),
            Instruction::Int128(data) => write!(f, "INT_128 {}", data.0),

            Instruction::UInt8(data) => write!(f, "UINT_8 {}", data.0),
            Instruction::UInt16(data) => write!(f, "UINT_16 {}", data.0),
            Instruction::UInt32(data) => write!(f, "UINT_32 {}", data.0),
            Instruction::UInt64(data) => write!(f, "UINT_64 {}", data.0),
            Instruction::UInt128(data) => write!(f, "UINT_128 {}", data.0),

            Instruction::Apply(count) => write!(f, "APPLY {}", count.arg_count),

            Instruction::BigInteger(data) => {
                write!(f, "BIG_INTEGER {}", data.0)
            }
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
            Instruction::ListStart => write!(f, "LIST_START"),
            Instruction::MapStart => write!(f, "MAP_START"),
            Instruction::StructStart => write!(f, "STRUCT_START"),
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
            Instruction::Union => write!(f, "UNION"),

            // equality checks
            Instruction::StructuralEqual => write!(f, "STRUCTURAL_EQUAL"),
            Instruction::Equal => write!(f, "EQUAL"),
            Instruction::NotStructuralEqual => {
                write!(f, "NOT_STRUCTURAL_EQUAL")
            }
            Instruction::NotEqual => write!(f, "NOT_EQUAL"),
            Instruction::Is => write!(f, "IS"),
            Instruction::Matches => write!(f, "MATCHES"),

            Instruction::AllocateSlot(address) => {
                write!(f, "ALLOCATE_SLOT {}", address.0)
            }
            Instruction::GetSlot(address) => {
                write!(f, "GET_SLOT {}", address.0)
            }
            Instruction::DropSlot(address) => {
                write!(f, "DROP_SLOT {}", address.0)
            }
            Instruction::SetSlot(address) => {
                write!(f, "SET_SLOT {}", address.0)
            }
            Instruction::AssignToReference(operator) => {
                write!(f, "ASSIGN_REFERENCE ({})", operator)
            }
            Instruction::Deref => write!(f, "DEREF"),
            Instruction::GetRef(address) => {
                write!(
                    f,
                    "GET_REF [{}:{}]",
                    address.endpoint,
                    hex::encode(address.id)
                )
            }
            Instruction::GetLocalRef(address) => {
                write!(
                    f,
                    "GET_LOCAL_REF [origin_id: {}]",
                    hex::encode(address.id)
                )
            }
            Instruction::GetInternalRef(address) => {
                write!(
                    f,
                    "GET_INTERNAL_REF [internal_id: {}]",
                    hex::encode(address.id)
                )
            }
            Instruction::CreateRef => write!(f, "CREATE_REF"),
            Instruction::CreateRefMut => write!(f, "CREATE_REF_MUT"),
            Instruction::CreateRefFinal => write!(f, "CREATE_REF_FINAL"),
            Instruction::GetOrCreateRef(data) => {
                write!(
                    f,
                    "GET_OR_CREATE_REF [{}:{}, block_size: {}]",
                    data.address.endpoint,
                    hex::encode(data.address.id),
                    data.create_block_size
                )
            }
            Instruction::GetOrCreateRefMut(data) => {
                write!(
                    f,
                    "GET_OR_CREATE_REF_MUT [{}:{}, block_size: {}]",
                    data.address.endpoint,
                    hex::encode(data.address.id),
                    data.create_block_size
                )
            }
            Instruction::ExecutionBlock(block) => {
                write!(
                    f,
                    "EXECUTION_BLOCK (length: {}, injected_slot_count: {})",
                    block.length, block.injected_slot_count
                )
            }
            Instruction::RemoteExecution => write!(f, "REMOTE_EXECUTION"),
            Instruction::AddAssign(address) => {
                write!(f, "ADD_ASSIGN {}", address.0)
            }
            Instruction::SubtractAssign(address) => {
                write!(f, "SUBTRACT_ASSIGN {}", address.0)
            }
            Instruction::MultiplyAssign(address) => {
                write!(f, "MULTIPLY_ASSIGN {}", address.0)
            }
            Instruction::DivideAssign(address) => {
                write!(f, "DIVIDE_ASSIGN {}", address.0)
            }
            Instruction::TypeInstructions(instr) => {
                let instr_strings: Vec<String> =
                    instr.iter().map(|i| i.to_string()).collect();
                write!(f, "TYPE_INSTRUCTIONS [{}]", instr_strings.join(", "))
            }
            Instruction::TypeExpression(instr) => {
                let instr_strings: Vec<String> =
                    instr.iter().map(|i| i.to_string()).collect();
                write!(f, "TYPE_EXPRESSION [{}]", instr_strings.join(", "))
            }
            Instruction::UnaryMinus => write!(f, "-"),
            Instruction::UnaryPlus => write!(f, "+"),
            Instruction::BitwiseNot => write!(f, "BITWISE_NOT"),
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeInstruction::LiteralText(data) => {
                write!(f, "LITERAL_TEXT {}", data.0)
            }
            TypeInstruction::LiteralInteger(data) => {
                write!(f, "LITERAL_INTEGER {}", data.0)
            }
            TypeInstruction::ListStart => write!(f, "LIST_START"),
            TypeInstruction::ScopeEnd => write!(f, "SCOPE_END"),
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

#[derive(BinRead, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct TypeTagVariant {
    pub length: u8,
    #[br(count = length)]
    #[br(map = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap())]
    pub name: String,
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
