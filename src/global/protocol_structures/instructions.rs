use crate::global::operators::AssignmentOperator;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::{
    decimal::utils::decimal_to_string, endpoint::Endpoint,
};
use binrw::{BinRead, BinWrite};
use core::fmt::Display;
use core::prelude::rust_2024::*;
use crate::global::type_instruction_codes::TypeMutabilityCode;

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    // regular instruction
    RegularInstruction(RegularInstruction),
    // Type instruction that yields a type
    TypeInstruction(TypeInstruction),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Instruction::RegularInstruction(instr) => core::write!(f, "{}", instr),
            Instruction::TypeInstruction(instr) => {
                core::write!(f, "TYPE_INSTRUCTION {}", instr)
            }
        }
    }
}

impl From<RegularInstruction> for Instruction {
    fn from(instruction: RegularInstruction) -> Self {
        Instruction::RegularInstruction(instruction)
    }
}

impl From<TypeInstruction> for Instruction {
    fn from(instruction: TypeInstruction) -> Self {
        Instruction::TypeInstruction(instruction)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegularInstruction {
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
    Statements(StatementsData),
    ShortStatements(ShortStatementsData),
    List(ListData),
    ShortList(ShortListData),
    Map(MapData),
    ShortMap(ShortMapData),

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
}

impl Display for RegularInstruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RegularInstruction::Int8(data) => core::write!(f, "INT_8 {}", data.0),
            RegularInstruction::Int16(data) => core::write!(f, "INT_16 {}", data.0),
            RegularInstruction::Int32(data) => core::write!(f, "INT_32 {}", data.0),
            RegularInstruction::Int64(data) => core::write!(f, "INT_64 {}", data.0),
            RegularInstruction::Int128(data) => core::write!(f, "INT_128 {}", data.0),

            RegularInstruction::UInt8(data) => core::write!(f, "UINT_8 {}", data.0),
            RegularInstruction::UInt16(data) => core::write!(f, "UINT_16 {}", data.0),
            RegularInstruction::UInt32(data) => core::write!(f, "UINT_32 {}", data.0),
            RegularInstruction::UInt64(data) => core::write!(f, "UINT_64 {}", data.0),
            RegularInstruction::UInt128(data) => {
                core::write!(f, "UINT_128 {}", data.0)
            }

            RegularInstruction::Apply(count) => {
                core::write!(f, "APPLY {}", count.arg_count)
            }

            RegularInstruction::BigInteger(data) => {
                core::write!(f, "BIG_INTEGER {}", data.0)
            }
            RegularInstruction::Endpoint(data) => {
                core::write!(f, "ENDPOINT {data}")
            }

            RegularInstruction::DecimalAsInt16(data) => {
                core::write!(f, "DECIMAL_AS_INT_16 {}", data.0)
            }
            RegularInstruction::DecimalAsInt32(data) => {
                core::write!(f, "DECIMAL_AS_INT_32 {}", data.0)
            }
            RegularInstruction::DecimalF32(data) => {
                core::write!(
                    f,
                    "DECIMAL_F32 {}",
                    decimal_to_string(data.0, false)
                )
            }
            RegularInstruction::DecimalF64(data) => {
                core::write!(
                    f,
                    "DECIMAL_F64 {}",
                    decimal_to_string(data.0, false)
                )
            }
            RegularInstruction::Decimal(data) => {
                core::write!(f, "DECIMAL_BIG {}", data.0)
            }
            RegularInstruction::ShortText(data) => {
                core::write!(f, "SHORT_TEXT {}", data.0)
            }
            RegularInstruction::Text(data) => core::write!(f, "TEXT {}", data.0),
            RegularInstruction::True => core::write!(f, "TRUE"),
            RegularInstruction::False => core::write!(f, "FALSE"),
            RegularInstruction::Null => core::write!(f, "NULL"),
            RegularInstruction::Statements(data) => {
                core::write!(f, "STATEMENTS {}", data.statements_count)
            }
            RegularInstruction::ShortStatements(data) => {
                core::write!(f, "SHORT_STATEMENTS {}", data.statements_count)
            }
            RegularInstruction::List(data) => {
                core::write!(f, "LIST {}", data.element_count)
            }
            RegularInstruction::ShortList(data) => {
                core::write!(f, "SHORT_LIST {}", data.element_count)
            }
            RegularInstruction::Map(data) => {
                core::write!(f, "MAP {}", data.element_count)
            }
            RegularInstruction::ShortMap(data) => {
                core::write!(f, "SHORT_MAP {}", data.element_count)
            }
            RegularInstruction::KeyValueDynamic => {
                core::write!(f, "KEY_VALUE_DYNAMIC")
            }
            RegularInstruction::KeyValueShortText(data) => {
                core::write!(f, "KEY_VALUE_SHORT_TEXT {}", data.0)
            }
            RegularInstruction::CloseAndStore => core::write!(f, "CLOSE_AND_STORE"),

            // operations
            RegularInstruction::Add => core::write!(f, "ADD"),
            RegularInstruction::Subtract => core::write!(f, "SUBTRACT"),
            RegularInstruction::Multiply => core::write!(f, "MULTIPLY"),
            RegularInstruction::Divide => core::write!(f, "DIVIDE"),

            // equality checks
            RegularInstruction::StructuralEqual => core::write!(f, "STRUCTURAL_EQUAL"),
            RegularInstruction::Equal => core::write!(f, "EQUAL"),
            RegularInstruction::NotStructuralEqual => {
                core::write!(f, "NOT_STRUCTURAL_EQUAL")
            }
            RegularInstruction::NotEqual => core::write!(f, "NOT_EQUAL"),
            RegularInstruction::Is => core::write!(f, "IS"),
            RegularInstruction::Matches => core::write!(f, "MATCHES"),

            RegularInstruction::AllocateSlot(address) => {
                core::write!(f, "ALLOCATE_SLOT {}", address.0)
            }
            RegularInstruction::GetSlot(address) => {
                core::write!(f, "GET_SLOT {}", address.0)
            }
            RegularInstruction::DropSlot(address) => {
                core::write!(f, "DROP_SLOT {}", address.0)
            }
            RegularInstruction::SetSlot(address) => {
                core::write!(f, "SET_SLOT {}", address.0)
            }
            RegularInstruction::AssignToReference(operator) => {
                core::write!(f, "ASSIGN_REFERENCE ({})", operator)
            }
            RegularInstruction::Deref => core::write!(f, "DEREF"),
            RegularInstruction::GetRef(address) => {
                core::write!(
                    f,
                    "GET_REF [{}:{}]",
                    address.endpoint,
                    hex::encode(address.id)
                )
            }
            RegularInstruction::GetLocalRef(address) => {
                core::write!(
                    f,
                    "GET_LOCAL_REF [origin_id: {}]",
                    hex::encode(address.id)
                )
            }
            RegularInstruction::GetInternalRef(address) => {
                core::write!(
                    f,
                    "GET_INTERNAL_REF [internal_id: {}]",
                    hex::encode(address.id)
                )
            }
            RegularInstruction::CreateRef => core::write!(f, "CREATE_REF"),
            RegularInstruction::CreateRefMut => core::write!(f, "CREATE_REF_MUT"),
            RegularInstruction::GetOrCreateRef(data) => {
                core::write!(
                    f,
                    "GET_OR_CREATE_REF [{}:{}, block_size: {}]",
                    data.address.endpoint,
                    hex::encode(data.address.id),
                    data.create_block_size
                )
            }
            RegularInstruction::GetOrCreateRefMut(data) => {
                core::write!(
                    f,
                    "GET_OR_CREATE_REF_MUT [{}:{}, block_size: {}]",
                    data.address.endpoint,
                    hex::encode(data.address.id),
                    data.create_block_size
                )
            }
            RegularInstruction::ExecutionBlock(block) => {
                core::write!(
                    f,
                    "EXECUTION_BLOCK (length: {}, injected_slot_count: {})",
                    block.length,
                    block.injected_slot_count
                )
            }
            RegularInstruction::RemoteExecution => core::write!(f, "REMOTE_EXECUTION"),
            RegularInstruction::AddAssign(address) => {
                core::write!(f, "ADD_ASSIGN {}", address.0)
            }
            RegularInstruction::SubtractAssign(address) => {
                core::write!(f, "SUBTRACT_ASSIGN {}", address.0)
            }
            RegularInstruction::MultiplyAssign(address) => {
                core::write!(f, "MULTIPLY_ASSIGN {}", address.0)
            }
            RegularInstruction::DivideAssign(address) => {
                core::write!(f, "DIVIDE_ASSIGN {}", address.0)
            }
            RegularInstruction::UnaryMinus => core::write!(f, "-"),
            RegularInstruction::UnaryPlus => core::write!(f, "+"),
            RegularInstruction::BitwiseNot => core::write!(f, "BITWISE_NOT"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeInstruction {
    ImplType(ImplTypeData),
    TypeReference(TypeReferenceData),
    LiteralText(TextData),
    LiteralInteger(IntegerData),
    List(ListData),
    // TODO: add more type instructions
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
            TypeInstruction::List(data) => {
                core::write!(f, "LIST {}", data.element_count)
            }
            TypeInstruction::TypeReference(address) => core::write!(
                f,
                "TYPE_REFERENCE",
            ),
            TypeInstruction::ImplType(data) => {
                core::write!(f, "IMPL_TYPE ({} impls)", data.impl_count)
            }
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
pub struct ShortListData {
    pub element_count: u8,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct StatementsData {
    pub statements_count: u32,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct ShortStatementsData {
    pub statements_count: u8,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct ListData {
    pub element_count: u32,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct ShortMapData {
    pub element_count: u8,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct MapData {
    pub element_count: u32,
}

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
    pub id: [u8; 26],
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
pub enum RawPointerAddress {
    #[br(magic = 120u8)] // InstructionCode::GET_REF
    Full(RawFullPointerAddress),
    #[br(magic = 121u8)] // InstructionCode::GET_INTERNAL_REF
    Internal(RawInternalPointerAddress),
    #[br(magic = 122u8)] // InstructionCode::GET_LOCAL_REF
    Local(RawLocalPointerAddress),
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
pub struct ImplTypeData {
    pub metadata: TypeMetadata,
    pub impl_count: u8,
    #[br(count = impl_count)]
    pub impls: Vec<RawPointerAddress>,
}

#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct TypeReferenceData {
    pub metadata: TypeMetadata,
    pub address: RawPointerAddress,
}


#[derive(BinRead, BinWrite, Clone, Debug, PartialEq)]
#[brw(little)]
pub struct TypeMetadata {
    pub mutability: TypeMutabilityCode,
}