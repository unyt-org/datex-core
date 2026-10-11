#[cfg(feature = "disassembler")]
pub mod debug;

#[cfg(feature = "disassembler")]
use crate::disassembler::InnerInstructions;
use crate::{
    dxb_parser::body::DXBParserError,
    global::{
        operators::{
            ArithmeticUnaryOperator, BinaryOperator, BitwiseUnaryOperator,
            ComparisonOperator, SharedValueUnaryOperator, UnaryOperator,
            binary::{ArithmeticOperator, RangeOperator},
        },
        root_properties::RootProperty,
        stack_index::StackIndex,
    },
    instruction::{
        NextExpectedInstructions,
        instruction_codes::InstructionCode,
        instruction_data::{
            ApplyData, Float32Data, Float64Data, FloatAsInt16Data,
            FloatAsInt32Data, InstantData, InstructionBlockData, Int8Data,
            Int16Data, Int32Data, Int64Data, Int128Data, JumpData,
            JumpWithValueData, ListData, MapData, MoveWithValue, SharedRef,
            SharedRefWithValue, ShortListData, ShortMapData,
            ShortStatementsData, ShortTextData, SpliceData, StatementsData,
            TaggedValue, TextData, UInt8Data, UInt16Data, UInt32Data,
            UInt64Data, UInt128Data, UnboundedStatementsData,
        },
    },
    libs::core::core_lib_id::CoreLibIdIndex,
    prelude::*,
    shared_values::{
        PointerAddress, ReferenceMutability, RemotePointerAddress,
        SelfOwnedPointerAddress,
    },
    values::core_values::{
        decimal::{Decimal, typed_decimal::TypedDecimal},
        endpoint::Endpoint,
        instant::Instant,
        integer::Integer,
    },
};
use binrw::{
    BinRead,
    io::{Read, Seek},
};
use core::fmt::{Display, Write as FmtWrite};
use datex_macros_internal::Instruction;

impl RegularInstruction {
    pub fn int8(value: i8) -> Self {
        RegularInstruction::Int8(Int8Data(value))
    }
    pub fn int16(value: i16) -> Self {
        RegularInstruction::Int16(Int16Data(value))
    }
    pub fn int32(value: i32) -> Self {
        RegularInstruction::Int32(Int32Data(value))
    }
    pub fn int64(value: i64) -> Self {
        RegularInstruction::Int64(Int64Data(value))
    }
    pub fn int128(value: i128) -> Self {
        RegularInstruction::Int128(Int128Data(value))
    }
    pub fn uint8(value: u8) -> Self {
        RegularInstruction::UInt8(UInt8Data(value))
    }
    pub fn uint16(value: u16) -> Self {
        RegularInstruction::UInt16(UInt16Data(value))
    }
    pub fn uint32(value: u32) -> Self {
        RegularInstruction::UInt32(UInt32Data(value))
    }
    pub fn uint64(value: u64) -> Self {
        RegularInstruction::UInt64(UInt64Data(value))
    }
    pub fn uint128(value: u128) -> Self {
        RegularInstruction::UInt128(UInt128Data(value))
    }
    pub fn decimal_f32(value: f32) -> Self {
        RegularInstruction::DecimalF32(Float32Data(value))
    }
    pub fn decimal_f64(value: f64) -> Self {
        RegularInstruction::DecimalF64(Float64Data(value))
    }
    pub fn decimal_as_int16(value: i16) -> Self {
        RegularInstruction::DecimalAsInt16(FloatAsInt16Data(value))
    }
    pub fn decimal_as_int32(value: i32) -> Self {
        RegularInstruction::DecimalAsInt32(FloatAsInt32Data(value))
    }
    pub fn decimal_big(value: Decimal) -> Self {
        RegularInstruction::BigDecimal(value)
    }
    pub fn decimal(value: Decimal) -> Self {
        RegularInstruction::Decimal(value)
    }
    pub fn integer(value: Integer) -> Self {
        RegularInstruction::Integer(value)
    }
    pub fn big_integer(value: Integer) -> Self {
        RegularInstruction::BigInteger(value)
    }
    pub fn endpoint(value: Endpoint) -> Self {
        RegularInstruction::Endpoint(value)
    }
    pub fn instant(value: i128) -> Self {
        RegularInstruction::Instant(InstantData(value))
    }

    /// Creates a text instruction, choosing between the short and default variant based on the length of the string.
    pub fn text(value: String) -> Self {
        match value.len() {
            0..=255 => Self::short_text(value),
            _ => Self::text_default(value),
        }
    }

    /// Creates a text instruction with the default variant, regardless of the length of the string.
    pub fn text_default(value: String) -> Self {
        RegularInstruction::Text(TextData(value))
    }

    /// Creates a short text instruction, regardless of the length of the string.
    pub fn short_text(value: String) -> Self {
        RegularInstruction::ShortText(ShortTextData(value))
    }

    pub fn tagged_value(tag: String, is_empty: bool) -> Self {
        RegularInstruction::TaggedValue(TaggedValue {
            tag: ShortTextData(tag),
            is_empty,
        })
    }

    /// Creates a list instruction, choosing between the short and default variant based on the count of elements.
    pub fn list(count: u32) -> Self {
        match count {
            0..=255 => Self::list_short(count as u8),
            _ => Self::list_default(count),
        }
    }

    /// Creates a list instruction with the default variant, regardless of the count of elements.
    pub fn list_default(count: u32) -> Self {
        RegularInstruction::List(ListData {
            element_count: count,
        })
    }

    /// Creates a list instruction with the short variant, regardless of the count of elements.
    pub fn list_short(count: u8) -> Self {
        RegularInstruction::ShortList(ShortListData {
            element_count: count,
        })
    }

    /// Creates a map instruction, choosing between the short and default variant based on the count of elements.
    pub fn map(count: u32) -> Self {
        match count {
            0..=255 => Self::map_short(count as u8),
            _ => Self::map_default(count),
        }
    }

    /// Creates a map instruction with the default variant, regardless of the count of elements.
    pub fn map_default(count: u32) -> Self {
        RegularInstruction::Map(MapData {
            element_count: count,
        })
    }

    /// Creates a map instruction with the short variant, regardless of the count of elements.
    pub fn map_short(count: u8) -> Self {
        RegularInstruction::ShortMap(ShortMapData {
            element_count: count,
        })
    }

    /// Creates a statements instruction, choosing between the short and default variant based on the count of statements.
    pub fn statements(count: u32, terminated: bool) -> Self {
        match count {
            0..=255 => Self::statements_short(count as u8, terminated),
            _ => Self::statements_default(count, terminated),
        }
    }

    /// Creates a statements instruction with the default variant, regardless of the count of statements.
    pub fn statements_default(count: u32, terminated: bool) -> Self {
        RegularInstruction::Statements(StatementsData {
            statements_count: count,
            terminated,
        })
    }

    /// Creates a statements instruction with the short variant, regardless of the count of statements.
    pub fn statements_short(count: u8, terminated: bool) -> Self {
        RegularInstruction::ShortStatements(ShortStatementsData {
            statements_count: count,
            terminated,
        })
    }

    pub fn unbounded_statements() -> Self {
        RegularInstruction::UnboundedStatements
    }
    pub fn unbounded_statements_end(terminated: bool) -> Self {
        RegularInstruction::UnboundedStatementsEnd(UnboundedStatementsData {
            terminated,
        })
    }

    /// Creates an apply instruction with the count of arguments.
    pub fn apply(arg_count: u8) -> Self {
        RegularInstruction::Apply(ApplyData { arg_count })
    }

    pub fn call_method(method_name: String, arg_count: u8) -> Self {
        RegularInstruction::CallMethod(CallMethodData {
            method_name: ShortTextData(method_name),
            arg_count,
        })
    }

    pub fn jump(offset: i32) -> Self {
        RegularInstruction::Jump(JumpData { offset })
    }

    pub fn jump_if_false(offset: i32) -> Self {
        RegularInstruction::JumpIfFalse(JumpData { offset })
    }

    pub fn get_entry_text(key: String) -> Self {
        RegularInstruction::GetEntryText(ShortTextData(key))
    }
    pub fn get_entry_index(index: u32) -> Self {
        RegularInstruction::GetEntryIndex(UInt32Data(index))
    }
    pub fn get_entry_dynamic() -> Self {
        RegularInstruction::GetEntryDynamic
    }
    pub fn set_entry_index(index: u32) -> Self {
        RegularInstruction::SetEntryIndex(UInt32Data(index))
    }
    pub fn set_entry_text(key: String) -> Self {
        RegularInstruction::SetEntryText(ShortTextData(key))
    }
    pub fn set_entry_dynamic() -> Self {
        RegularInstruction::SetEntryDynamic
    }
    pub fn matches() -> Self {
        RegularInstruction::Matches
    }
    pub fn structural_equal() -> Self {
        RegularInstruction::StructuralEqual
    }
    pub fn not_structural_equal() -> Self {
        RegularInstruction::NotStructuralEqual
    }
    pub fn equal() -> Self {
        RegularInstruction::Equal
    }
    pub fn not_equal() -> Self {
        RegularInstruction::NotEqual
    }
    pub fn is() -> Self {
        RegularInstruction::Is
    }
    pub fn add() -> Self {
        RegularInstruction::Add
    }
    pub fn subtract() -> Self {
        RegularInstruction::Subtract
    }
    pub fn multiply() -> Self {
        RegularInstruction::Multiply
    }
    pub fn divide() -> Self {
        RegularInstruction::Divide
    }
    pub fn unary_operation(operator: UnaryOperator) -> Self {
        match operator {
            UnaryOperator::Arithmetic(op) => match op {
                ArithmeticUnaryOperator::Plus => RegularInstruction::UnaryPlus,
                ArithmeticUnaryOperator::Minus => {
                    RegularInstruction::UnaryMinus
                }
                ArithmeticUnaryOperator::Decrement => {
                    RegularInstruction::Decrement
                }
                ArithmeticUnaryOperator::Increment => {
                    RegularInstruction::Increment
                }
            },
            UnaryOperator::Bitwise(op) => match op {
                BitwiseUnaryOperator::Not => RegularInstruction::BitwiseNot,
            },
            UnaryOperator::Reference(op) => match op {
                SharedValueUnaryOperator::Unbox => RegularInstruction::Unbox,
            },
            UnaryOperator::Logical(_) => {
                todo!("Logical unary operators not implemented yet")
            }
        }
    }
    pub fn binary_operation(operator: BinaryOperator) -> Self {
        match operator {
            BinaryOperator::Arithmetic(op) => match op {
                ArithmeticOperator::Add => RegularInstruction::Add,
                ArithmeticOperator::Subtract => RegularInstruction::Subtract,
                ArithmeticOperator::Multiply => RegularInstruction::Multiply,
                ArithmeticOperator::Divide => RegularInstruction::Divide,
                ArithmeticOperator::Modulo => {
                    todo!("Modulo binary operator not implemented yet")
                }
                ArithmeticOperator::Power => {
                    todo!("Power binary operator not implemented yet")
                }
            },
            BinaryOperator::Bitwise(_) => {
                todo!("Bitwise binary operators not implemented yet")
            }
            BinaryOperator::Logical(_) => {
                todo!("Logical binary operators not implemented yet")
            }
            BinaryOperator::Range(op) => match op {
                RangeOperator::Inclusive => RegularInstruction::Range,
                RangeOperator::Exclusive => {
                    todo!("Exclusive range operator not implemented yet")
                }
            },
        }
    }

    pub fn comparison_operation(operator: ComparisonOperator) -> Self {
        match operator {
            ComparisonOperator::Is => RegularInstruction::Is,
            ComparisonOperator::Matches => RegularInstruction::Matches,
            ComparisonOperator::StructuralEqual => {
                RegularInstruction::StructuralEqual
            }
            ComparisonOperator::NotStructuralEqual => {
                RegularInstruction::NotStructuralEqual
            }
            ComparisonOperator::Equal => RegularInstruction::Equal,
            ComparisonOperator::NotEqual => RegularInstruction::NotEqual,
            ComparisonOperator::LessThan => todo!(),
            ComparisonOperator::GreaterThan => todo!(),
            ComparisonOperator::LessThanOrEqual => todo!(),
            ComparisonOperator::GreaterThanOrEqual => todo!(),
        }
    }

    pub fn unary_plus() -> Self {
        RegularInstruction::UnaryPlus
    }
    pub fn unary_minus() -> Self {
        RegularInstruction::UnaryMinus
    }
    pub fn bitwise_not() -> Self {
        RegularInstruction::BitwiseNot
    }
    pub fn increment() -> Self {
        RegularInstruction::Increment
    }
    pub fn decrement() -> Self {
        RegularInstruction::Decrement
    }
    pub fn append_entry() -> Self {
        RegularInstruction::AppendEntry
    }
    pub fn clear() -> Self {
        RegularInstruction::Clear
    }
    pub fn splice(
        start_index: u32,
        delete_count: u32,
        insert_count: u32,
    ) -> Self {
        RegularInstruction::Splice(SpliceData {
            start_index,
            delete_count,
            insert_count,
        })
    }
    pub fn splice_dynamic() -> Self {
        RegularInstruction::SpliceDynamic
    }
    pub fn set_shared_container_value() -> Self {
        RegularInstruction::SetSharedContainerValue
    }
    pub fn take_entry_text(key: String) -> Self {
        RegularInstruction::TakeEntryText(ShortTextData(key))
    }
    pub fn take_entry_index(index: u32) -> Self {
        RegularInstruction::TakeEntryIndex(UInt32Data(index))
    }
    pub fn take_entry_dynamic() -> Self {
        RegularInstruction::TakeEntryDynamic
    }
    pub fn null() -> Self {
        RegularInstruction::Null
    }
    pub fn r#true() -> Self {
        RegularInstruction::True
    }
    pub fn r#false() -> Self {
        RegularInstruction::False
    }
    pub fn boolean(value: bool) -> Self {
        if value {
            Self::r#true()
        } else {
            Self::r#false()
        }
    }
    pub fn set_stack_value(stack_index: StackIndex) -> Self {
        RegularInstruction::SetStackValue(stack_index)
    }
    pub fn borrow_stack_value(stack_index: StackIndex) -> Self {
        RegularInstruction::BorrowStackValue(stack_index)
    }
    pub fn clone_stack_value(stack_index: StackIndex) -> Self {
        RegularInstruction::CloneStackValue(stack_index)
    }
    pub fn key_value_dynamic() -> Self {
        RegularInstruction::KeyValueDynamic
    }
    pub fn key_value_short_text(key: String) -> Self {
        RegularInstruction::KeyValueShortText(ShortTextData(key))
    }
    pub fn push_to_stack() -> Self {
        RegularInstruction::PushToStack
    }
    pub fn push_list_to_stack() -> Self {
        RegularInstruction::PushListToStack
    }
    pub fn get_stack_value_shared_ref(stack_index: StackIndex) -> Self {
        RegularInstruction::GetStackValueSharedRef(stack_index)
    }
    pub fn get_stack_value_shared_ref_mut(stack_index: StackIndex) -> Self {
        RegularInstruction::GetStackValueSharedRefMut(stack_index)
    }
    pub fn take_stack_value(stack_index: StackIndex) -> Self {
        RegularInstruction::TakeStackValue(stack_index)
    }
    pub fn get_root_property(root_property: RootProperty) -> Self {
        RegularInstruction::GetRootProperty(root_property)
    }
    pub fn unbox() -> Self {
        RegularInstruction::Unbox
    }
    pub fn type_expression() -> Self {
        RegularInstruction::TypeExpression
    }
    pub fn derive_shared_reference() -> Self {
        RegularInstruction::DeriveSharedReference
    }
    pub fn derive_shared_reference_mut() -> Self {
        RegularInstruction::DeriveSharedReferenceMut
    }
    pub fn create_shared() -> Self {
        RegularInstruction::CreateShared
    }
    pub fn create_shared_mut() -> Self {
        RegularInstruction::CreateSharedMut
    }
    pub fn request_remote_shared_ref(address: RemotePointerAddress) -> Self {
        RegularInstruction::RequestRemoteSharedRef(address)
    }
    pub fn request_remote_shared_ref_mut(
        address: RemotePointerAddress,
    ) -> Self {
        RegularInstruction::RequestRemoteSharedRefMut(address)
    }
    pub fn get_local_shared_ref(address: SelfOwnedPointerAddress) -> Self {
        RegularInstruction::GetLocalSharedRef(address)
    }
    pub fn get_core_lib_value(core_lib_id: CoreLibIdIndex) -> Self {
        RegularInstruction::GetCoreLibValue(core_lib_id)
    }
    pub fn shared_ref(shared_ref: SharedRef) -> Self {
        RegularInstruction::SharedRef(shared_ref)
    }
    pub fn shared_ref_with_value(
        shared_ref_with_value: SharedRefWithValue,
    ) -> Self {
        RegularInstruction::SharedRefWithValue(shared_ref_with_value)
    }
    pub fn move_with_value(move_with_value: MoveWithValue) -> Self {
        RegularInstruction::MoveWithValue(move_with_value)
    }
    pub fn remote_execution(instruction_block: InstructionBlockData) -> Self {
        RegularInstruction::RemoteExecution(instruction_block)
    }
    pub fn range() -> Self {
        RegularInstruction::Range
    }
    pub fn boxed_value() -> Self {
        RegularInstruction::BoxedValue
    }

    pub fn get_shared_ref(
        address: PointerAddress,
        mutability: &ReferenceMutability,
    ) -> Self {
        match address {
            PointerAddress::SelfOwned(local_address) => {
                RegularInstruction::get_local_shared_ref(local_address)
            }
            PointerAddress::Remote(address) => match mutability {
                ReferenceMutability::Immutable => {
                    RegularInstruction::request_remote_shared_ref(address)
                }
                ReferenceMutability::Mutable => {
                    RegularInstruction::request_remote_shared_ref_mut(address)
                }
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Instruction)]
pub enum RegularInstruction {
    #[magic(InstructionCode::UNINITIALIZED)]
    Uninitialized,

    // signed integers
    #[magic(InstructionCode::INT_8)]
    Int8(Int8Data),
    #[magic(InstructionCode::INT_16)]
    Int16(Int16Data),
    #[magic(InstructionCode::INT_32)]
    Int32(Int32Data),
    #[magic(InstructionCode::INT_64)]
    Int64(Int64Data),
    #[magic(InstructionCode::INT_128)]
    Int128(Int128Data),

    // unsigned integers
    #[magic(InstructionCode::UINT_8)]
    UInt8(UInt8Data),
    #[magic(InstructionCode::UINT_16)]
    UInt16(UInt16Data),
    #[magic(InstructionCode::UINT_32)]
    UInt32(UInt32Data),
    #[magic(InstructionCode::UINT_64)]
    UInt64(UInt64Data),
    #[magic(InstructionCode::UINT_128)]
    UInt128(UInt128Data),

    // big integers
    #[magic(InstructionCode::INT_BIG)]
    BigInteger(Integer),

    // default integer
    #[magic(InstructionCode::INT)]
    Integer(Integer),
    #[magic(InstructionCode::RANGE)]
    Range,

    #[magic(InstructionCode::BOXED_VALUE)]
    BoxedValue,

    #[magic(InstructionCode::ENDPOINT)]
    Endpoint(Endpoint),

    #[magic(InstructionCode::INSTANT)]
    Instant(InstantData),

    #[magic(InstructionCode::DECIMAL_F32)]
    DecimalF32(Float32Data),
    #[magic(InstructionCode::DECIMAL_F64)]
    DecimalF64(Float64Data),
    #[magic(InstructionCode::DECIMAL_AS_INT_16)]
    DecimalAsInt16(FloatAsInt16Data),
    #[magic(InstructionCode::DECIMAL_AS_INT_32)]
    DecimalAsInt32(FloatAsInt32Data),
    #[magic(InstructionCode::DECIMAL_BIG)]
    BigDecimal(Decimal),
    // default decimal
    #[magic(InstructionCode::DECIMAL)]
    Decimal(Decimal),

    #[magic(InstructionCode::REMOTE_EXECUTION)]
    RemoteExecution(InstructionBlockData),
    #[magic(InstructionCode::SHORT_TEXT)]
    ShortText(ShortTextData),
    #[magic(InstructionCode::TEXT)]
    Text(TextData),

    #[magic(InstructionCode::TRUE)]
    True,
    #[magic(InstructionCode::FALSE)]
    False,
    #[magic(InstructionCode::NULL)]
    Null,
    #[magic(InstructionCode::STATEMENTS)]
    Statements(StatementsData),
    #[magic(InstructionCode::SHORT_STATEMENTS)]
    ShortStatements(ShortStatementsData),
    #[magic(InstructionCode::UNBOUNDED_STATEMENTS)]
    UnboundedStatements,
    #[magic(InstructionCode::UNBOUNDED_STATEMENTS_END)]
    UnboundedStatementsEnd(UnboundedStatementsData),
    #[magic(InstructionCode::LIST)]
    List(ListData),
    #[magic(InstructionCode::SHORT_LIST)]
    ShortList(ShortListData),
    #[magic(InstructionCode::MAP)]
    Map(MapData),
    #[magic(InstructionCode::SHORT_MAP)]
    ShortMap(ShortMapData),

    #[magic(InstructionCode::KEY_VALUE_DYNAMIC)]
    KeyValueDynamic,
    #[magic(InstructionCode::KEY_VALUE_SHORT_TEXT)]
    KeyValueShortText(ShortTextData),

    #[magic(InstructionCode::TAGGED_VALUE)]
    TaggedValue(TaggedValue),

    // binary operator
    #[magic(InstructionCode::ADD)]
    Add,
    #[magic(InstructionCode::SUBTRACT)]
    Subtract,
    #[magic(InstructionCode::MULTIPLY)]
    Multiply,
    #[magic(InstructionCode::DIVIDE)]
    Divide,

    // unary operator
    // TODO #432 add missing unary operators
    #[magic(InstructionCode::UNARY_MINUS)]
    UnaryMinus,
    // TODO #433: Do we need this for op overloading or can we avoid?
    #[magic(InstructionCode::UNARY_PLUS)]
    UnaryPlus,
    #[magic(InstructionCode::BITWISE_NOT)]
    BitwiseNot,

    #[magic(InstructionCode::APPLY)]
    Apply(ApplyData),

    #[magic(InstructionCode::CALL_METHOD)]
    CallMethod(CallMethodData),

    #[magic(InstructionCode::GET_ENTRY_TEXT)]
    GetEntryText(ShortTextData),

    #[magic(InstructionCode::GET_ENTRY_INDEX)]
    GetEntryIndex(UInt32Data),

    #[magic(InstructionCode::GET_ENTRY_DYNAMIC)]
    GetEntryDynamic,

    // Jumps
    #[magic(InstructionCode::JUMP)]
    Jump(JumpData),
    #[magic(InstructionCode::JUMP_IF_FALSE)]
    JumpIfFalse(JumpData),
    #[magic(InstructionCode::JUMP_WITH_VALUE)]
    JumpWithValue(JumpWithValueData),

    // comparison operator
    #[magic(InstructionCode::IS)]
    Is,
    #[magic(InstructionCode::MATCHES)]
    Matches,
    #[magic(InstructionCode::STRUCTURAL_EQUAL)]
    StructuralEqual,
    #[magic(InstructionCode::EQUAL)]
    Equal,
    #[magic(InstructionCode::NOT_STRUCTURAL_EQUAL)]
    NotStructuralEqual,
    #[magic(InstructionCode::NOT_EQUAL)]
    NotEqual,

    #[magic(InstructionCode::DERIVE_SHARED_REF)]
    DeriveSharedReference,
    #[magic(InstructionCode::DERIVE_SHARED_REF_MUT)]
    DeriveSharedReferenceMut,

    #[magic(InstructionCode::CREATE_SHARED)]
    CreateShared,
    #[magic(InstructionCode::CREATE_SHARED_MUT)]
    CreateSharedMut,

    // ' $ABCDE
    #[magic(InstructionCode::REQUEST_REMOTE_SHARED_REF)]
    RequestRemoteSharedRef(RemotePointerAddress),
    // 'mut $ABCDE
    #[magic(InstructionCode::REQUEST_REMOTE_SHARED_REF_MUT)]
    RequestRemoteSharedRefMut(RemotePointerAddress),
    #[magic(InstructionCode::GET_LOCAL_SHARED_REF)]
    GetLocalSharedRef(SelfOwnedPointerAddress),
    // get a core lib value, e.g. integer or print by id
    #[magic(InstructionCode::GET_CORE_LIB_VALUE)]
    GetCoreLibValue(CoreLibIdIndex),

    #[magic(InstructionCode::SHARED_REF)]
    SharedRef(SharedRef),
    #[magic(InstructionCode::SHARED_REF_WITH_VALUE)]
    SharedRefWithValue(SharedRefWithValue),

    #[magic(InstructionCode::MOVE_WITH_VALUE)]
    MoveWithValue(MoveWithValue),

    #[magic(InstructionCode::PUSH_TO_STACK)]
    PushToStack,
    #[magic(InstructionCode::PUSH_LIST_TO_STACK)]
    PushListToStack,
    #[magic(InstructionCode::CLONE_STACK_VALUE)]
    CloneStackValue(StackIndex),
    #[magic(InstructionCode::BORROW_STACK_VALUE)]
    BorrowStackValue(StackIndex),
    #[magic(InstructionCode::GET_STACK_VALUE_SHARED_REF)]
    GetStackValueSharedRef(StackIndex),
    #[magic(InstructionCode::GET_STACK_VALUE_SHARED_REF_MUT)]
    GetStackValueSharedRefMut(StackIndex),
    #[magic(InstructionCode::TAKE_STACK_VALUE)]
    TakeStackValue(StackIndex),
    #[magic(InstructionCode::SET_STACK_VALUE)]
    SetStackValue(StackIndex),

    #[magic(InstructionCode::GET_ROOT_PROPERTY)]
    GetRootProperty(RootProperty),

    #[magic(InstructionCode::UNBOX)]
    Unbox,

    #[magic(InstructionCode::CALLABLE_DECLARATION)]
    CallableDeclaration(CallableDeclarationData),

    #[magic(InstructionCode::CALLABLE)]
    Callable(CallableData),

    #[magic(InstructionCode::ENTITY_VALUE)]
    EntityValue(PointerAddress),
    #[magic(InstructionCode::TYPE_EXPRESSION)]
    TypeExpression,

    // modification instructions: will later be mapped to trait impls
    // UpdateOperation::Replace
    #[magic(InstructionCode::SET_SHARED_CONTAINER_VALUE)]
    SetSharedContainerValue,

    // UpdateOperation::AppendEntry
    #[magic(InstructionCode::APPEND_ENTRY)]
    AppendEntry,
    // UpdateOperation::Clear
    #[magic(InstructionCode::CLEAR)]
    Clear,
    // UpdateOperation::Splice
    #[magic(InstructionCode::SPLICE)]
    Splice(SpliceData),
    #[magic(InstructionCode::SPLICE_DYNAMIC)]
    SpliceDynamic,

    // UpdateOperation::Increment
    #[magic(InstructionCode::INCREMENT)]
    Increment,
    // UpdateOperation::Decrement
    #[magic(InstructionCode::DECREMENT)]
    Decrement,

    // UpdateOperation::DeleteEntry
    #[magic(InstructionCode::TAKE_ENTRY_TEXT)]
    TakeEntryText(ShortTextData),
    #[magic(InstructionCode::TAKE_ENTRY_INDEX)]
    TakeEntryIndex(UInt32Data),
    #[magic(InstructionCode::TAKE_ENTRY_DYNAMIC)]
    TakeEntryDynamic,

    // UpdateOperation::SetEntry
    #[magic(InstructionCode::SET_ENTRY_TEXT)]
    SetEntryText(ShortTextData),
    #[magic(InstructionCode::SET_ENTRY_INDEX)]
    SetEntryIndex(UInt32Data),
    #[magic(InstructionCode::SET_ENTRY_DYNAMIC)]
    SetEntryDynamic,

    /// Debug variant for RemoteExecution, includes full remote execution instruction list (flat) instead of raw dxb
    /// This variant is only used by the disassembler
    #[cfg(feature = "disassembler")]
    #[instruction(skip)]
    _RemoteExecutionDebugFlat(
        crate::instruction::instruction_data::InstructionBlockDataDebugFlat,
    ),
    /// Debug variant for RemoteExecution, includes full remote execution instruction tree instead of raw dxb
    /// This variant is only used by the disassembler
    #[cfg(feature = "disassembler")]
    #[instruction(skip)]
    _RemoteExecutionDebugTree(
        crate::instruction::instruction_data::InstructionBlockDataDebugTree,
    ),

    /// Debug variant for [CallableDeclarationData], includes full remote execution instruction list (flat) instead of raw dxb
    /// This variant is only used by the disassembler
    #[cfg(feature = "disassembler")]
    #[instruction(skip)]
    _CallableDeclarationDebugFlat(
        crate::instruction::instruction_data::CallableDeclarationDataDebugFlat,
    ),
    /// Debug variant for [CallableDeclarationData], includes full remote execution instruction tree instead of raw dxb
    /// This variant is only used by the disassembler
    #[cfg(feature = "disassembler")]
    #[instruction(skip)]
    _CallableDeclarationDebugTree(
        crate::instruction::instruction_data::CallableDeclarationDataDebugTree,
    ),

    /// Debug variant for [CallableData], includes full remote execution instruction list (flat) instead of raw dxb
    /// This variant is only used by the disassembler
    #[cfg(feature = "disassembler")]
    #[instruction(skip)]
    _CallableDebugFlat(
        crate::instruction::instruction_data::CallableDataDebugFlat,
    ),
    /// Debug variant for [CallableData], includes full remote execution instruction tree instead of raw dxb
    /// This variant is only used by the disassembler
    #[cfg(feature = "disassembler")]
    #[instruction(skip)]
    _CallableDebugTree(
        crate::instruction::instruction_data::CallableDataDebugTree,
    ),
}

impl RegularInstruction {
    pub fn instruction_code_string(&self) -> String {
        if let Some(code) = self.code() {
            code.to_string()
        } else {
            #[cfg(feature = "disassembler")]
            if let Some(code) = self.debug_instruction_code() {
                return code.to_string();
            }

            "?".to_string()
        }
    }

    /// Returns how many (if any) regular or type instructions are expected as child instructions for a given instructions
    pub fn get_next_expected_instructions(&self) -> NextExpectedInstructions {
        match self {
            RegularInstruction::RemoteExecution(_) => {
                NextExpectedInstructions::Regular(1)
            } // receivers

            #[cfg(feature = "disassembler")]
            RegularInstruction::_RemoteExecutionDebugTree(_)
            | RegularInstruction::_RemoteExecutionDebugFlat(_) => {
                NextExpectedInstructions::Regular(1)
            } // receivers

            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDeclarationDebugFlat(data) => {
                NextExpectedInstructions::Type(
                    data.signature.total_type_count(),
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDeclarationDebugTree(data) => {
                NextExpectedInstructions::Type(
                    data.signature.total_type_count(),
                )
            }

            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDebugFlat(data) => {
                NextExpectedInstructions::RegularAndType(
                    data.body.injected_value_count,
                    data.signature.total_type_count(),
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDebugTree(data) => {
                NextExpectedInstructions::RegularAndType(
                    data.body.injected_value_count,
                    data.signature.total_type_count(),
                )
            }

            RegularInstruction::ShortList(list) => {
                NextExpectedInstructions::Regular(list.element_count as u32)
            } // list elements

            RegularInstruction::List(list) => {
                NextExpectedInstructions::Regular(list.element_count)
            } // list elements

            RegularInstruction::ShortMap(map) => {
                NextExpectedInstructions::Regular(map.element_count as u32)
            } // map entries

            RegularInstruction::Map(map) => {
                NextExpectedInstructions::Regular(map.element_count)
            } // map entries

            RegularInstruction::ShortStatements(statements) => {
                NextExpectedInstructions::Regular(
                    statements.statements_count as u32,
                )
            }
            RegularInstruction::Statements(statements) => {
                NextExpectedInstructions::Regular(statements.statements_count)
            } // statements in block

            RegularInstruction::UnboundedStatements => {
                NextExpectedInstructions::UnboundedStart
            }

            RegularInstruction::UnboundedStatementsEnd(_) => {
                NextExpectedInstructions::UnboundedEnd
            }

            RegularInstruction::Apply(apply_data) => {
                NextExpectedInstructions::Regular(
                    apply_data.arg_count as u32 + 1,
                )
            } // arguments plus base to apply to

            RegularInstruction::CallMethod(call_method_data) => {
                NextExpectedInstructions::Regular(
                    call_method_data.arg_count as u32 + 1,
                )
            } // arguments plus base to call method on

            RegularInstruction::GetEntryText(_)
            | RegularInstruction::GetEntryIndex(_)
            | RegularInstruction::TakeEntryText(_)
            | RegularInstruction::TakeEntryIndex(_) => {
                NextExpectedInstructions::Regular(1)
            } // value to get property from

            RegularInstruction::GetEntryDynamic
            | RegularInstruction::TakeEntryDynamic => {
                NextExpectedInstructions::Regular(2)
            } // value to get property from + property key

            RegularInstruction::SetEntryText(_)
            | RegularInstruction::SetEntryIndex(_) => {
                NextExpectedInstructions::Regular(2)
            } // value to set property on and new value

            RegularInstruction::SetEntryDynamic => {
                NextExpectedInstructions::Regular(3)
            } // value to set property on + property key + new value

            RegularInstruction::Unbox => NextExpectedInstructions::Regular(1), // value to unbox

            RegularInstruction::AppendEntry => {
                NextExpectedInstructions::Regular(2)
            }
            RegularInstruction::Splice(SpliceData { insert_count, .. }) => {
                NextExpectedInstructions::Regular(*insert_count + 1)
            }
            RegularInstruction::SpliceDynamic => {
                NextExpectedInstructions::Regular(4)
            }

            RegularInstruction::SetSharedContainerValue => {
                NextExpectedInstructions::Regular(2)
            } // container to set value on + new value

            RegularInstruction::KeyValueDynamic => {
                NextExpectedInstructions::Regular(2)
            } // key + value

            RegularInstruction::KeyValueShortText(_) => {
                NextExpectedInstructions::Regular(1)
            } // value

            RegularInstruction::Matches => {
                NextExpectedInstructions::RegularAndType(1, 1)
            }

            RegularInstruction::Add
            | RegularInstruction::Multiply
            | RegularInstruction::Subtract
            | RegularInstruction::Divide => {
                NextExpectedInstructions::Regular(2)
            } // left and right operand

            RegularInstruction::StructuralEqual
            | RegularInstruction::NotStructuralEqual
            | RegularInstruction::Equal
            | RegularInstruction::NotEqual
            | RegularInstruction::Is => NextExpectedInstructions::Regular(2), // left and right operand

            RegularInstruction::UnaryMinus
            | RegularInstruction::UnaryPlus
            | RegularInstruction::BitwiseNot => {
                NextExpectedInstructions::Regular(1)
            }

            RegularInstruction::DeriveSharedReference
            | RegularInstruction::DeriveSharedReferenceMut
            | RegularInstruction::CreateShared
            | RegularInstruction::CreateSharedMut => {
                NextExpectedInstructions::Regular(1)
            }

            RegularInstruction::PushToStack
            | RegularInstruction::PushListToStack
            | RegularInstruction::SetStackValue(_) => {
                NextExpectedInstructions::Regular(1)
            }
            RegularInstruction::EntityValue(_) => {
                NextExpectedInstructions::Regular(1)
            }

            RegularInstruction::TypeExpression => {
                NextExpectedInstructions::Type(1)
            }
            RegularInstruction::Jump(_) => NextExpectedInstructions::None,
            RegularInstruction::JumpIfFalse(_) => {
                NextExpectedInstructions::Regular(1)
            }
            RegularInstruction::Range => NextExpectedInstructions::Regular(2),
            RegularInstruction::TaggedValue(TaggedValue {
                is_empty, ..
            }) => {
                if *is_empty {
                    NextExpectedInstructions::None
                } else {
                    NextExpectedInstructions::Regular(1)
                }
            }

            RegularInstruction::SharedRefWithValue(_) => {
                NextExpectedInstructions::Regular(1)
            }
            RegularInstruction::MoveWithValue(_) => {
                NextExpectedInstructions::Regular(1)
            }

            RegularInstruction::Increment => {
                NextExpectedInstructions::Regular(2)
            }
            RegularInstruction::Decrement => {
                NextExpectedInstructions::Regular(2)
            }
            RegularInstruction::BoxedValue => {
                NextExpectedInstructions::Regular(1)
            }
            RegularInstruction::CallableDeclaration(data) => {
                NextExpectedInstructions::Type(
                    data.signature.total_type_count(),
                )
            }
            RegularInstruction::Callable(data) => {
                NextExpectedInstructions::RegularAndType(
                    data.body.injected_value_count,
                    data.signature.total_type_count(),
                )
            }

            _ => NextExpectedInstructions::None,
        }
    }

    fn read_regular_instruction_code<R: Read + Seek>(
        mut reader: &mut R,
    ) -> Result<InstructionCode, DXBParserError> {
        let instruction_code = u8::read(&mut reader)
            .map_err(|_| DXBParserError::FailedToReadInstructionCode)?;

        InstructionCode::try_from(instruction_code).map_err(|_| {
            DXBParserError::InvalidInstructionCode(instruction_code)
        })
    }

    pub fn metadata_string(&self) -> Option<String> {
        let mut string = String::new();

        match self {
            RegularInstruction::Int8(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Int16(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Int32(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Int64(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Int128(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::UInt8(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::UInt16(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::UInt32(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::UInt64(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::UInt128(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Apply(count) => {
                write!(string, "[arg_count: {}]", count.arg_count)
            }
            RegularInstruction::Jump(offset) => {
                write!(string, "offset: {}", offset.offset)
            }
            RegularInstruction::JumpIfFalse(offset) => {
                write!(string, "offset: {}", offset.offset)
            }
            RegularInstruction::CallMethod(data) => {
                write!(string, "[method_name: {}, arg_count: {}]", data.method_name.0, data.arg_count)
            }
            RegularInstruction::BigInteger(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Integer(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Endpoint(data) => {
                write!(string, "{data}")
            }
            RegularInstruction::Instant(data) => {
                write!(string, "{}", Instant(data.0).to_iso_string())
            }

            RegularInstruction::DecimalAsInt16(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::DecimalAsInt32(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::DecimalF32(data) => {
                write!(
                    string,
                    "{}",
                    TypedDecimal::F32(data.0.into())
                )
            }
            RegularInstruction::DecimalF64(data) => {
                write!(
                    string,
                    "{}",
                    TypedDecimal::F64(data.0.into())
                )
            }
            RegularInstruction::BigDecimal(data) => {
                write!(string, "{}", data)
            }
            RegularInstruction::Decimal(data) => {
                write!(string, "{}", data)
            }
            RegularInstruction::ShortText(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Text(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::Statements(data) => {
                write!(string, "[count: {}, terminated: {}]", data.statements_count, data.terminated)
            }
            RegularInstruction::ShortStatements(data) => {
                write!(string, "[count: {}, terminated: {}]", data.statements_count, data.terminated)
            }
            RegularInstruction::List(data) => {
                write!(string, "{}", data.element_count)
            }
            RegularInstruction::ShortList(data) => {
                write!(string, "{}", data.element_count)
            }
            RegularInstruction::Map(data) => {
                write!(string, "{}", data.element_count)
            }
            RegularInstruction::ShortMap(data) => {
                write!(string, "{}", data.element_count)
            }
            RegularInstruction::KeyValueShortText(data) => {
                write!(string, "{}", data.0)
            }
            RegularInstruction::CloneStackValue(address) => {
                write!(string, "{}", address.0)
            }
            RegularInstruction::GetRootProperty(property) => {
                write!(string, "$.{}", property)
            }
            RegularInstruction::BorrowStackValue(address) => {
                write!(string, "{}", address.0)
            }
            RegularInstruction::GetStackValueSharedRef(address) => {
                write!(string, "{}", address.0)
            }
            RegularInstruction::GetStackValueSharedRefMut(address) => {
                write!(string, "{}", address.0)
            }
            RegularInstruction::TakeStackValue(address) => {
                write!(string, "{}", address.0)
            }
            RegularInstruction::SetStackValue(address) => {
                write!(string, "{}", address.0)
            }
            RegularInstruction::Splice(splice_data) => {
                write!(string, "[start: {}, delete_count: {}, insert_count: {}]", splice_data.start_index, splice_data.delete_count, splice_data.insert_count)
            }
            RegularInstruction::RequestRemoteSharedRef(address) => {
                write!(
                    string,
                    "[endpoint: {}, address:{}]",
                    address.endpoint(),
                    address
                )
            }
            RegularInstruction::RequestRemoteSharedRefMut(address) => {
                write!(
                    string,
                    "[endpoint: {}, address:{}]",
                    address.endpoint(),
                    address
                )
            }
            RegularInstruction::GetLocalSharedRef(address) => {
                write!(
                    string,
                    "[address:{}]",
                    address
                )
            }
            RegularInstruction::GetCoreLibValue(address) => {
                write!(
                    string,
                    "[id: {:4x}]",
                    &address.0
                )
            }
            RegularInstruction::SharedRef(shared_ref) => {
                write!(
                    string,
                    "[ref_mutability: {:?}, address: {}]",
                    shared_ref.ref_mutability, shared_ref.address.clone()
                )
            }
            RegularInstruction::SharedRefWithValue(shared_ref) => {
                write!(
                    string,
                    "[ref_mutability: {:?}, address: {}, container_mutability: {:?}]",
                    shared_ref.ref_mutability,
                    PointerAddress::from(shared_ref.address.clone()),
                    shared_ref.container_mutability
                )
            }
            RegularInstruction::MoveWithValue(move_with_value) => {
                write!(
                    string,
                    "[mutability: {:?}, previous address: {}]",
                    move_with_value.mutability,
                    move_with_value.previous_address
                )
            }
            RegularInstruction::RemoteExecution(data) => {
                write!(
                    string,
                    "{}",
                    data
                )
            }
            RegularInstruction::CallableDeclaration(data) => {
                write!(
                    string,
                    "[signature: {}, body: {}]",
                    data.signature,
                    data.body
                )
            }
            RegularInstruction::Callable(data) => {
                write!(
                    string,
                    "[signature: {}, body: {}]",
                    data.signature,
                    data.body
                )
            }
            RegularInstruction::GetEntryIndex(uint_32_data) => {
                write!(string, "{}", uint_32_data.0)
            }
            RegularInstruction::SetEntryIndex(uint_32_data) => {
                write!(string, "{}", uint_32_data.0)
            }
            RegularInstruction::TakeEntryIndex(uint_32_data) => {
                write!(string, "{}", uint_32_data.0)
            }
            RegularInstruction::GetEntryText(short_text_data) => {
                write!(string, "{}", short_text_data.0)
            }
            RegularInstruction::TakeEntryText(short_text_data) => {
                write!(string, "{}", short_text_data.0)
            }
            RegularInstruction::SetEntryText(short_text_data) => {
                write!(string, "{}", short_text_data.0)
            }

            RegularInstruction::TaggedValue(tagged_value) => {
                write!(
                    string,
                    "[tag: {}, is_empty: {}]",
                    tagged_value.tag.0, tagged_value.is_empty
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDeclarationDebugTree(data) => {
                write!(
                    string,
                    "[signature: {}, body: {}]",
                    data.signature,
                    data.body
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDeclarationDebugFlat(data) => {
                write!(
                    string,
                    "[signature: {}, body: {}]",
                    data.signature,
                    data.body
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDebugTree(data) => {
                write!(
                    string,
                    "[signature: {}, body: {}]",
                    data.signature,
                    data.body
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_CallableDebugFlat(data) => {
                write!(
                    string,
                    "[signature: {}, body: {}]",
                    data.signature,
                    data.body
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_RemoteExecutionDebugTree(data) => {
                write!(
                    string,
                    "{}",
                    data
                )
            }
            #[cfg(feature = "disassembler")]
            RegularInstruction::_RemoteExecutionDebugFlat(data) => {
                write!(
                    string,
                    "{}",
                    data
                )
            }

            _ => {
                // no custom disassembly
                return None;
            }
        }.unwrap();

        Some(string)
    }
}

use crate::instruction::instruction_data::{
    CallMethodData, CallableData, CallableDeclarationData,
};
/// Serializes RegularInstruction to tuple (instruction code as string, optional metadata as string)
#[cfg(feature = "disassembler")]
use serde::{Serialize, Serializer, ser::SerializeTuple};

#[cfg(feature = "disassembler")]
impl Serialize for RegularInstruction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let instruction_code = self.instruction_code_string();
        let metadata_string = self.metadata_string();

        if let Some(metadata_string) = metadata_string {
            let inner_instructions =
                self.inner_instructions_from_debug_instruction();
            let count = if inner_instructions == InnerInstructions::None {
                2
            } else {
                3
            };

            let mut state = serializer.serialize_tuple(count)?;
            state.serialize_element(&instruction_code)?;
            state.serialize_element(&metadata_string)?;
            match inner_instructions {
                InnerInstructions::Tree(data) => {
                    state.serialize_element(&data)?
                }
                InnerInstructions::Flat(data) => {
                    state.serialize_element(&data)?
                }
                InnerInstructions::None => {}
            }
            state.end()
        } else {
            serializer.serialize_str(&instruction_code)
        }
    }
}

impl Display for RegularInstruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(code) = self.code() {
            write!(f, "{}", code)?;
        }

        if let Some(metadata_string) = self.metadata_string() {
            write!(f, " {}", metadata_string)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use binrw::{BinResult, BinWrite};

    use super::*;
    use binrw::io::Cursor;

    fn encode(instruction: &RegularInstruction) -> Vec<u8> {
        let mut writer = Cursor::new(Vec::new());
        instruction
            .write(&mut writer)
            .expect("instruction should serialize");
        writer.into_inner()
    }

    fn decode(bytes: &[u8]) -> BinResult<RegularInstruction> {
        let mut reader = Cursor::new(bytes);
        let instruction = RegularInstruction::read(&mut reader)?;
        assert_eq!(
            reader.position() as usize,
            bytes.len(),
            "decoder did not consume the complete instruction"
        );
        Ok(instruction)
    }

    fn assert_round_trip(instruction: RegularInstruction) {
        let encoded = encode(&instruction);
        let decoded =
            decode(&encoded).expect("encoded instruction should deserialize");

        assert_eq!(decoded, instruction);
    }

    #[test]
    fn int8() {
        let instruction = RegularInstruction::int8(42);
        let encoded = encode(&instruction);
        assert_eq!(encoded, vec![InstructionCode::INT_8.as_u8(), 42u8,]);
    }

    #[test]
    fn negative_int8() {
        let instruction = RegularInstruction::int8(-5);
        let encoded = encode(&instruction);
        assert_eq!(
            encoded,
            vec![InstructionCode::INT_8.as_u8(), (-5i8) as u8,]
        );
        assert_eq!(decode(&encoded).unwrap(), instruction,);
    }

    #[test]
    fn int16() {
        let instruction = RegularInstruction::int16(0x1234);
        let encoded = encode(&instruction);
        assert_eq!(encoded, vec![InstructionCode::INT_16.as_u8(), 0x34, 0x12,]);
    }

    #[test]
    fn instruction_code_matches_variant() {
        assert_eq!(
            RegularInstruction::int8(1).code().unwrap(),
            InstructionCode::INT_8,
        );
        assert_eq!(
            RegularInstruction::int16(1).code().unwrap(),
            InstructionCode::INT_16,
        );
        assert_eq!(
            RegularInstruction::Int32(Int32Data(1),).code().unwrap(),
            InstructionCode::INT_32,
        );
        assert_eq!(
            RegularInstruction::UInt8(UInt8Data(1),).code().unwrap(),
            InstructionCode::UINT_8,
        );
    }

    #[test]
    fn integer_variants_round_trip() {
        let instructions = [
            RegularInstruction::Int8(Int8Data(i8::MIN)),
            RegularInstruction::Int8(Int8Data(i8::MAX)),
            RegularInstruction::Int16(Int16Data(i16::MIN)),
            RegularInstruction::Int16(Int16Data(i16::MAX)),
            RegularInstruction::Int32(Int32Data(i32::MIN)),
            RegularInstruction::Int32(Int32Data(i32::MAX)),
            RegularInstruction::Int64(Int64Data(i64::MIN)),
            RegularInstruction::Int64(Int64Data(i64::MAX)),
            RegularInstruction::Int128(Int128Data(i128::MIN)),
            RegularInstruction::Int128(Int128Data(i128::MAX)),
        ];

        for instruction in instructions {
            assert_round_trip(instruction);
        }
    }

    #[test]
    fn unsigned_integer_variants_round_trip() {
        let instructions = [
            RegularInstruction::UInt8(UInt8Data(u8::MAX)),
            RegularInstruction::UInt16(UInt16Data(u16::MAX)),
            RegularInstruction::UInt32(UInt32Data(u32::MAX)),
            RegularInstruction::UInt64(UInt64Data(u64::MAX)),
            RegularInstruction::UInt128(UInt128Data(u128::MAX)),
        ];

        for instruction in instructions {
            assert_round_trip(instruction);
        }
    }

    #[test]
    fn stream() {
        let instructions = vec![
            RegularInstruction::int8(-12),
            RegularInstruction::int16(1234),
            RegularInstruction::UInt32(UInt32Data(987_654)),
        ];
        let mut stream = Cursor::new(Vec::new());
        for instruction in &instructions {
            instruction
                .write(&mut stream)
                .expect("instruction should serialize");
        }

        stream.set_position(0);

        let decoded: Vec<_> = (0..instructions.len())
            .map(|_| {
                RegularInstruction::read(&mut stream)
                    .expect("instruction should deserialize")
            })
            .collect();

        assert_eq!(decoded, instructions);
        assert_eq!(stream.position() as usize, stream.get_ref().len(),);
    }

    #[test]
    fn missing_bytes() {
        let bytes = [
            InstructionCode::INT_16.as_u8(),
            0x34,
            // Missing the second i16 byte
        ];

        let result = RegularInstruction::read(&mut Cursor::new(bytes));
        assert!(result.is_err());
    }

    #[test]
    fn unknown_opcode() {
        let bytes = [0xff];
        let result = RegularInstruction::read(&mut Cursor::new(bytes));
        assert!(result.is_err());
    }

    #[test]
    fn reencoding_decoded() {
        let original =
            vec![InstructionCode::INT_32.as_u8(), 0x78, 0x56, 0x34, 0x12];
        let decoded = decode(&original).expect("valid INT_32");
        let reencoded = encode(&decoded);
        assert_eq!(reencoded, original);
    }
}
