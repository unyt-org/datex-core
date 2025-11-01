use crate::decompiler::ScopeType;
use crate::global::instruction_codes::InstructionCode;
use crate::global::protocol_structures::instructions::{
    ApplyData, DecimalData, ExecutionBlockData, Float32Data, Float64Data,
    FloatAsInt16Data, FloatAsInt32Data, Instruction, Int8Data, Int16Data,
    Int32Data, Int64Data, Int128Data, IntegerData, RawFullPointerAddress,
    RawInternalPointerAddress, ShortTextData, ShortTextDataRaw, SlotAddress,
    TextData, TextDataRaw, TypeInstruction, UInt8Data, UInt16Data, UInt32Data,
    UInt64Data, UInt128Data,
};
use crate::global::type_instruction_codes::TypeSpaceInstructionCode;
use core::fmt;
use crate::utils::buffers;
use crate::values::core_values::endpoint::Endpoint;
use binrw::BinRead;
use datex_core::ast::structs::operator::assignment::AssignmentOperator;
use datex_core::global::protocol_structures::instructions::RawLocalPointerAddress;
use core::fmt::Display;
use crate::stdlib::io::{BufRead, Cursor, Read, Seek};

fn extract_scope(dxb_body: &[u8], index: &mut usize) -> Vec<u8> {
    let size = buffers::read_u32(dxb_body, index);
    buffers::read_vec_slice(dxb_body, index, size as usize)
}

#[derive(Debug)]
pub enum DXBParserError {
    InvalidEndpoint(String),
    InvalidBinaryCode(u8),
    FailedToReadInstructionCode,
    FmtError(fmt::Error),
    BinRwError(binrw::Error),
    FromUtf8Error(std::string::FromUtf8Error),
    InvalidScopeEndType {
        expected: ScopeType,
        found: ScopeType,
    },
}

impl From<fmt::Error> for DXBParserError {
    fn from(error: fmt::Error) -> Self {
        DXBParserError::FmtError(error)
    }
}

impl From<binrw::Error> for DXBParserError {
    fn from(error: binrw::Error) -> Self {
        DXBParserError::BinRwError(error)
    }
}

impl From<std::string::FromUtf8Error> for DXBParserError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        DXBParserError::FromUtf8Error(error)
    }
}

impl Display for DXBParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DXBParserError::InvalidBinaryCode(code) => {
                core::write!(f, "Invalid binary code: {code}")
            }
            DXBParserError::InvalidEndpoint(endpoint) => {
                core::write!(f, "Invalid endpoint: {endpoint}")
            }
            DXBParserError::FailedToReadInstructionCode => {
                core::write!(f, "Failed to read instruction code")
            }
            DXBParserError::FmtError(err) => {
                core::write!(f, "Formatting error: {err}")
            }
            DXBParserError::BinRwError(err) => {
                core::write!(f, "Binary read/write error: {err}")
            }
            DXBParserError::FromUtf8Error(err) => {
                core::write!(f, "UTF-8 conversion error: {err}")
            }
            DXBParserError::InvalidScopeEndType { expected, found } => {
                core::write!(
                    f,
                    "Invalid scope end type: expected {expected:?}, found {found:?}"
                )
            }
        }
    }
}

fn get_short_text_data(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<ShortTextData, DXBParserError> {
    let raw_data = ShortTextDataRaw::read(&mut reader);
    if let Err(err) = raw_data {
        Err(err.into())
    } else {
        let raw_data = raw_data?;
        let text = String::from_utf8(raw_data.text);
        if let Err(err) = text {
            Err(err.into())
        } else {
            let text = text?;
            Ok(ShortTextData(text))
        }
    }
}

fn get_endpoint_data(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<Endpoint, DXBParserError> {
    let raw_data = Endpoint::read(&mut reader);
    if let Ok(endpoint) = raw_data {
        Ok(endpoint)
    } else {
        Err(raw_data.err().unwrap().into())
    }
}

fn get_text_data(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<TextData, DXBParserError> {
    let raw_data = TextDataRaw::read(&mut reader);
    if let Err(err) = raw_data {
        Err(err.into())
    } else {
        let raw_data = raw_data?;
        let text = String::from_utf8(raw_data.text);
        if let Err(err) = text {
            Err(err.into())
        } else {
            let text = text?;
            Ok(TextData(text))
        }
    }
}

// TODO #221: refactor: pass a ParserState struct instead of individual parameters
pub fn iterate_instructions<'a>(
    dxb_body: &'a [u8],
) -> impl Iterator<Item = Result<Instruction, DXBParserError>> + 'a {
    core::iter::from_coroutine(
        #[coroutine]
        move || {
            // get reader for dxb_body
            let mut reader = Cursor::new(dxb_body);
            loop {
                // if cursor is at the end, break
                // rationale: We can use safe unwrap here, as our stream is no IO, but only
                // bytes stream, so we can always access.
                unsafe {
                    if !reader.has_data_left().unwrap_unchecked() {
                        return;
                    }
                }

                let instruction_code = get_next_instruction_code(&mut reader);
                if let Err(err) = instruction_code {
                    yield Err(err);
                    return;
                }
                let instruction_code = instruction_code.unwrap();

                yield match instruction_code {
                    // TODO #425: Refactor with macros to reduce code duplication
                    InstructionCode::UINT_8 => {
                        let data = UInt8Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::UInt8(data.unwrap()))
                        }
                    }
                    InstructionCode::UINT_16 => {
                        let data = UInt16Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::UInt16(data.unwrap()))
                        }
                    }
                    InstructionCode::UINT_32 => {
                        let data = UInt32Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::UInt32(data.unwrap()))
                        }
                    }
                    InstructionCode::UINT_64 => {
                        let data = UInt64Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::UInt64(data.unwrap()))
                        }
                    }
                    InstructionCode::UINT_128 => {
                        let data = UInt128Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::UInt128(data.unwrap()))
                        }
                    }

                    InstructionCode::INT_8 => {
                        let data = Int8Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::Int8(data.unwrap()))
                        }
                    }

                    InstructionCode::INT_16 => {
                        let data = Int16Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::Int16(data.unwrap()))
                        }
                    }

                    InstructionCode::INT_32 => {
                        let data = Int32Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::Int32(data.unwrap()))
                        }
                    }

                    InstructionCode::INT_64 => {
                        let data = Int64Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::Int64(data.unwrap()))
                        }
                    }

                    InstructionCode::INT_128 => {
                        let data = Int128Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::Int128(data.unwrap()))
                        }
                    }

                    InstructionCode::INT_BIG => {
                        let data = IntegerData::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::BigInteger(data.unwrap()))
                        }
                    }

                    InstructionCode::DECIMAL_F32 => {
                        let data = Float32Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::DecimalF32(data.unwrap()))
                        }
                    }
                    InstructionCode::DECIMAL_F64 => {
                        let data = Float64Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::DecimalF64(data.unwrap()))
                        }
                    }

                    InstructionCode::DECIMAL_BIG => {
                        let data = DecimalData::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::Decimal(data.unwrap()))
                        }
                    }

                    InstructionCode::DECIMAL_AS_INT_16 => {
                        let data = FloatAsInt16Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::DecimalAsInt16(data.unwrap()))
                        }
                    }

                    InstructionCode::DECIMAL_AS_INT_32 => {
                        let data = FloatAsInt32Data::read(&mut reader);
                        if let Err(err) = data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::DecimalAsInt32(data.unwrap()))
                        }
                    }

                    InstructionCode::REMOTE_EXECUTION => {
                        Ok(Instruction::RemoteExecution)
                    }
                    InstructionCode::EXECUTION_BLOCK => {
                        ExecutionBlockData::read(&mut reader)
                            .map(Instruction::ExecutionBlock)
                            .map_err(|err| err.into())
                    }

                    InstructionCode::SHORT_TEXT => {
                        get_short_text_data(&mut reader)
                            .map(Instruction::ShortText)
                    }

                    InstructionCode::ENDPOINT => get_endpoint_data(&mut reader)
                        .map(Instruction::Endpoint),

                    InstructionCode::TEXT => {
                        let text_data = get_text_data(&mut reader);
                        if let Err(err) = text_data {
                            Err(err)
                        } else {
                            Ok(Instruction::Text(text_data.unwrap()))
                        }
                    }

                    InstructionCode::TRUE => Ok(Instruction::True),
                    InstructionCode::FALSE => Ok(Instruction::False),
                    InstructionCode::NULL => Ok(Instruction::Null),

                    // complex terms
                    InstructionCode::LIST_START => Ok(Instruction::ListStart),
                    InstructionCode::MAP_START => Ok(Instruction::MapStart),
                    InstructionCode::SCOPE_START => Ok(Instruction::ScopeStart),
                    InstructionCode::SCOPE_END => Ok(Instruction::ScopeEnd),

                    InstructionCode::APPLY_ZERO => {
                        Ok(Instruction::Apply(ApplyData { arg_count: 0 }))
                    }
                    InstructionCode::APPLY_SINGLE => {
                        Ok(Instruction::Apply(ApplyData { arg_count: 1 }))
                    }

                    InstructionCode::APPLY => {
                        let apply_data = ApplyData::read(&mut reader);
                        if let Err(err) = apply_data {
                            Err(err.into())
                        } else {
                            Ok(Instruction::Apply(apply_data.unwrap()))
                        }
                    }

                    InstructionCode::DEREF => Ok(Instruction::Deref),
                    InstructionCode::ASSIGN_TO_REF => {
                        let operator = get_next_instruction_code(&mut reader);
                        if let Err(err) = operator {
                            yield Err(err);
                            return;
                        }
                        let operator =
                            AssignmentOperator::try_from(operator.unwrap())
                                .map_err(|_| {
                                    DXBParserError::InvalidBinaryCode(
                                        instruction_code as u8,
                                    )
                                });
                        if let Err(err) = operator {
                            yield Err(err);
                            return;
                        }
                        Ok(Instruction::AssignToReference(operator.unwrap()))
                    }

                    InstructionCode::KEY_VALUE_SHORT_TEXT => {
                        let short_text_data = get_short_text_data(&mut reader);
                        if let Err(err) = short_text_data {
                            Err(err)
                        } else {
                            Ok(Instruction::KeyValueShortText(
                                short_text_data.unwrap(),
                            ))
                        }
                    }

                    InstructionCode::KEY_VALUE_DYNAMIC => {
                        Ok(Instruction::KeyValueDynamic)
                    }

                    InstructionCode::CLOSE_AND_STORE => {
                        Ok(Instruction::CloseAndStore)
                    }

                    // operations
                    InstructionCode::ADD => Ok(Instruction::Add),
                    InstructionCode::SUBTRACT => Ok(Instruction::Subtract),
                    InstructionCode::MULTIPLY => Ok(Instruction::Multiply),
                    InstructionCode::DIVIDE => Ok(Instruction::Divide),

                    InstructionCode::UNARY_MINUS => Ok(Instruction::UnaryMinus),
                    InstructionCode::UNARY_PLUS => Ok(Instruction::UnaryPlus),
                    InstructionCode::BITWISE_NOT => Ok(Instruction::BitwiseNot),

                    // equality
                    InstructionCode::STRUCTURAL_EQUAL => {
                        Ok(Instruction::StructuralEqual)
                    }
                    InstructionCode::EQUAL => Ok(Instruction::Equal),
                    InstructionCode::NOT_STRUCTURAL_EQUAL => {
                        Ok(Instruction::NotStructuralEqual)
                    }
                    InstructionCode::NOT_EQUAL => Ok(Instruction::NotEqual),
                    InstructionCode::IS => Ok(Instruction::Is),
                    InstructionCode::MATCHES => Ok(Instruction::Matches),
                    InstructionCode::CREATE_REF => Ok(Instruction::CreateRef),
                    InstructionCode::CREATE_REF_MUT => {
                        Ok(Instruction::CreateRefMut)
                    }

                    // slots
                    InstructionCode::ALLOCATE_SLOT => {
                        let address = SlotAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::AllocateSlot(address.unwrap()))
                        }
                    }
                    InstructionCode::GET_SLOT => {
                        let address = SlotAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::GetSlot(address.unwrap()))
                        }
                    }
                    InstructionCode::DROP_SLOT => {
                        let address = SlotAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::DropSlot(address.unwrap()))
                        }
                    }
                    InstructionCode::SET_SLOT => {
                        let address = SlotAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::SetSlot(address.unwrap()))
                        }
                    }

                    InstructionCode::GET_REF => {
                        let address = RawFullPointerAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::GetRef(address.unwrap()))
                        }
                    }

                    InstructionCode::GET_LOCAL_REF => {
                        let address = RawLocalPointerAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::GetLocalRef(address.unwrap()))
                        }
                    }

                    InstructionCode::GET_INTERNAL_REF => {
                        let address =
                            RawInternalPointerAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::GetInternalRef(address.unwrap()))
                        }
                    }

                    InstructionCode::ADD_ASSIGN => {
                        let address = SlotAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::AddAssign(address.unwrap()))
                        }
                    }

                    InstructionCode::SUBTRACT_ASSIGN => {
                        let address = SlotAddress::read(&mut reader);
                        if let Err(err) = address {
                            Err(err.into())
                        } else {
                            Ok(Instruction::SubtractAssign(address.unwrap()))
                        }
                    }

                    InstructionCode::TYPED_VALUE => {
                        // collect type space instructions
                        let result: Result<
                            Vec<TypeInstruction>,
                            DXBParserError,
                        > = iterate_type_space_instructions(&mut reader)
                            .collect();
                        if let Err(err) = result {
                            Err(err)
                        } else {
                            Ok(Instruction::TypeInstructions(result.unwrap()))
                        }
                    }
                    InstructionCode::TYPE_EXPRESSION => {
                        // collect type space instructions
                        let result: Result<
                            Vec<TypeInstruction>,
                            DXBParserError,
                        > = iterate_type_space_instructions(&mut reader)
                            .collect();
                        if let Err(err) = result {
                            Err(err)
                        } else {
                            Ok(Instruction::TypeExpression(result.unwrap()))
                        }
                    }

                    _ => Err(DXBParserError::InvalidBinaryCode(
                        instruction_code as u8,
                    )),
                }
            }
        },
    )
}

fn get_next_instruction_code(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<InstructionCode, DXBParserError> {
    let instruction_code = u8::read(&mut reader)
        .map_err(|err| DXBParserError::FailedToReadInstructionCode)?;

    InstructionCode::try_from(instruction_code)
        .map_err(|_| DXBParserError::FailedToReadInstructionCode)
}

fn iterate_type_space_instructions<R: Read + Seek + BufRead>(
    reader: &mut R,
) -> impl Iterator<Item = Result<TypeInstruction, DXBParserError>> {
    core::iter::from_coroutine(
        #[coroutine]
        move || {
            loop {
                // if cursor is at the end, break
                unsafe {
                    // rationale: We can use safe unwrap here, as our stream is no IO, but only
                    // bytes stream, so we can always access.
                    if !reader.has_data_left().unwrap_unchecked() {
                        return;
                    }
                }

                let instruction_code = u8::read(reader);
                if let Err(err) = instruction_code {
                    yield Err(err.into());
                    return;
                }

                let instruction_code = TypeSpaceInstructionCode::try_from(
                    instruction_code.unwrap(),
                );
                if instruction_code.is_err() {
                    yield Err(DXBParserError::FailedToReadInstructionCode);
                    return;
                }
                let instruction_code = instruction_code.unwrap();
                //info!("Instruction code: {:?}", instruction_code);

                yield match instruction_code {
                    TypeSpaceInstructionCode::TYPE_LIST_START => {
                        Ok(TypeInstruction::ListStart)
                    }
                    TypeSpaceInstructionCode::TYPE_LITERAL_INTEGER => {
                        let integer_data = IntegerData::read(reader);
                        if let Err(err) = integer_data {
                            Err(err.into())
                        } else {
                            Ok(TypeInstruction::LiteralInteger(
                                integer_data.unwrap(),
                            ))
                        }
                    }
                    _ => todo!("#426 Undescribed by author."),
                }
            }
        },
    )
}
