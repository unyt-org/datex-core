use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::assignment::AssignmentOperator;
use crate::global::protocol_structures::instructions::{ApplyData, DecimalData, ExecutionBlockData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, RegularInstruction, Int8Data, Int16Data, Int32Data, Int64Data, Int128Data, IntegerData, RawFullPointerAddress, RawInternalPointerAddress, ShortTextData, ShortTextDataRaw, SlotAddress, TextData, TextDataRaw, TypeInstruction, UInt8Data, UInt16Data, UInt32Data, UInt64Data, UInt128Data, ImplTypeData, RawPointerAddress, TypeReferenceData, Instruction};
use crate::global::type_instruction_codes::TypeInstructionCode;
use crate::stdlib::string::FromUtf8Error;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::utils::buffers;
use crate::values::core_values::endpoint::Endpoint;
use binrw::BinRead;
use binrw::io::Cursor;
use core::fmt;
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;
use log::info;
use datex_core::global::protocol_structures::instructions::RawLocalPointerAddress;
use datex_core::parser::next_instructions_stack::NextInstructionsStack;
use crate::parser::next_instructions_stack::NextInstructionType;
use crate::runtime::execution::macros::yield_unwrap;
use crate::stdlib::format;
use crate::stdlib::convert::TryFrom;

fn extract_scope(dxb_body: &[u8], index: &mut usize) -> Vec<u8> {
    let size = buffers::read_u32(dxb_body, index);
    buffers::read_vec_slice(dxb_body, index, size as usize)
}

#[derive(Debug)]
pub enum DXBParserError {
    InvalidEndpoint(String),
    InvalidBinaryCode(u8),
    FailedToReadInstructionCode,
    InvalidInstructionCode(u8),
    FmtError(fmt::Error),
    BinRwError(binrw::Error),
    FromUtf8Error(FromUtf8Error),
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

impl From<FromUtf8Error> for DXBParserError {
    fn from(error: FromUtf8Error) -> Self {
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
            DXBParserError::InvalidInstructionCode(code) => {
                core::write!(f, "Encountered an invalid instruction code: {:2X}", code)
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
        }
    }
}

pub fn iterate_instructions<'a>(
    dxb_body: &'a [u8],
    next_instructions_stack: &'a mut NextInstructionsStack,
) -> impl Iterator<Item = Result<Instruction, DXBParserError>> + 'a {

    // debug log bytes
    info!(
        "DXB Body Bytes: {}",
        dxb_body
            .chunks(16)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|byte| format!("{:02X}", byte))
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect::<Vec<String>>()
            .join("\n")
    );

    core::iter::from_coroutine(
        #[coroutine]
        move || {
            // get reader for dxb_body
            let len = dxb_body.len();
            let mut reader = Cursor::new(dxb_body);
            loop {
                // if cursor is at the end, break
                if reader.position() as usize >= len {
                    return;
                }

                let next_instruction_type = next_instructions_stack.pop();

                yield Ok(match next_instruction_type {

                    NextInstructionType::End => return, // end of instructions

                    NextInstructionType::Regular => {
                        let instruction_code = yield_unwrap!(get_next_regular_instruction_code(&mut reader));

                        match instruction_code {
                            InstructionCode::UINT_8 => {
                                let data = UInt8Data::read(&mut reader);
                                RegularInstruction::UInt8(yield_unwrap!(data))
                            }
                            InstructionCode::UINT_16 => {
                                let data = UInt16Data::read(&mut reader);
                                RegularInstruction::UInt16(yield_unwrap!(data))
                            }
                            InstructionCode::UINT_32 => {
                                let data = UInt32Data::read(&mut reader);
                                RegularInstruction::UInt32(yield_unwrap!(data))
                            }
                            InstructionCode::UINT_64 => {
                                let data = UInt64Data::read(&mut reader);
                                RegularInstruction::UInt64(yield_unwrap!(data))
                            }
                            InstructionCode::UINT_128 => {
                                let data = UInt128Data::read(&mut reader);
                                RegularInstruction::UInt128(yield_unwrap!(data))
                            }

                            InstructionCode::INT_8 => {
                                let data = Int8Data::read(&mut reader);
                                RegularInstruction::Int8(yield_unwrap!(data))
                            }
                            InstructionCode::INT_16 => {
                                let data = Int16Data::read(&mut reader);
                                RegularInstruction::Int16(yield_unwrap!(data))
                            }
                            InstructionCode::INT_32 => {
                                let data = Int32Data::read(&mut reader);
                                RegularInstruction::Int32(yield_unwrap!(data))
                            }
                            InstructionCode::INT_64 => {
                                let data = Int64Data::read(&mut reader);
                                RegularInstruction::Int64(yield_unwrap!(data))
                            }
                            InstructionCode::INT_128 => {
                                let data = Int128Data::read(&mut reader);
                                RegularInstruction::Int128(yield_unwrap!(data))
                            }
                            InstructionCode::INT_BIG => {
                                let data = IntegerData::read(&mut reader);
                                RegularInstruction::BigInteger(yield_unwrap!(data))
                            }

                            InstructionCode::DECIMAL_F32 => {
                                let data = Float32Data::read(&mut reader);
                                RegularInstruction::DecimalF32(yield_unwrap!(data))
                            }
                            InstructionCode::DECIMAL_F64 => {
                                let data = Float64Data::read(&mut reader);
                                RegularInstruction::DecimalF64(yield_unwrap!(data))
                            }
                            InstructionCode::DECIMAL_BIG => {
                                let data = DecimalData::read(&mut reader);
                                RegularInstruction::Decimal(yield_unwrap!(data))
                            }
                            InstructionCode::DECIMAL_AS_INT_16 => {
                                let data = FloatAsInt16Data::read(&mut reader);
                                RegularInstruction::DecimalAsInt16(yield_unwrap!(data))
                            }
                            InstructionCode::DECIMAL_AS_INT_32 => {
                                let data = FloatAsInt32Data::read(&mut reader);
                                RegularInstruction::DecimalAsInt32(yield_unwrap!(data))
                            }

                            InstructionCode::REMOTE_EXECUTION => RegularInstruction::RemoteExecution,
                            InstructionCode::EXECUTION_BLOCK => {
                                let data = ExecutionBlockData::read(&mut reader);
                                RegularInstruction::ExecutionBlock(yield_unwrap!(data))
                            }

                            InstructionCode::SHORT_TEXT => {
                                let raw_data = ShortTextDataRaw::read(&mut reader);
                                let text = yield_unwrap!(String::from_utf8(yield_unwrap!(raw_data).text));
                                RegularInstruction::ShortText(ShortTextData(text))
                            }

                            InstructionCode::ENDPOINT => {
                                let endpoint_data = Endpoint::read(&mut reader);
                                RegularInstruction::Endpoint(yield_unwrap!(endpoint_data))
                            }

                            InstructionCode::TEXT => {
                                let raw_data = TextDataRaw::read(&mut reader);
                                let text = yield_unwrap!(String::from_utf8(yield_unwrap!(raw_data).text));
                                RegularInstruction::Text(TextData(text))
                            }

                            InstructionCode::TRUE => RegularInstruction::True,
                            InstructionCode::FALSE => RegularInstruction::False,
                            InstructionCode::NULL => RegularInstruction::Null,

                            // complex terms
                            InstructionCode::LIST_START => RegularInstruction::ListStart,
                            InstructionCode::MAP_START => RegularInstruction::MapStart,
                            InstructionCode::SCOPE_START => RegularInstruction::ScopeStart,
                            InstructionCode::SCOPE_END => RegularInstruction::ScopeEnd,

                            InstructionCode::APPLY_ZERO => RegularInstruction::Apply(ApplyData { arg_count: 0 }),
                            InstructionCode::APPLY_SINGLE => RegularInstruction::Apply(ApplyData { arg_count: 1 }),

                            InstructionCode::APPLY => {
                                let apply_data = ApplyData::read(&mut reader);
                                RegularInstruction::Apply(yield_unwrap!(apply_data))
                            }

                            InstructionCode::DEREF => RegularInstruction::Deref,
                            InstructionCode::ASSIGN_TO_REF => {
                                let operator = yield_unwrap!(get_next_regular_instruction_code(&mut reader));
                                let operator = yield_unwrap!(
                                    AssignmentOperator::try_from(operator)
                                        .map_err(|_| {
                                            DXBParserError::InvalidBinaryCode(
                                                instruction_code as u8,
                                            )
                                        })
                                );
                                RegularInstruction::AssignToReference(operator)
                            }

                            InstructionCode::KEY_VALUE_SHORT_TEXT => {
                                let raw_data = ShortTextDataRaw::read(&mut reader);
                                let text = yield_unwrap!(String::from_utf8(yield_unwrap!(raw_data).text));
                                RegularInstruction::KeyValueShortText(ShortTextData(text))
                            }

                            InstructionCode::KEY_VALUE_DYNAMIC => RegularInstruction::KeyValueDynamic,
                            InstructionCode::CLOSE_AND_STORE => RegularInstruction::CloseAndStore,

                            // operations
                            InstructionCode::ADD => RegularInstruction::Add,
                            InstructionCode::SUBTRACT => RegularInstruction::Subtract,
                            InstructionCode::MULTIPLY => RegularInstruction::Multiply,
                            InstructionCode::DIVIDE => RegularInstruction::Divide,

                            InstructionCode::UNARY_MINUS => RegularInstruction::UnaryMinus,
                            InstructionCode::UNARY_PLUS => RegularInstruction::UnaryPlus,
                            InstructionCode::BITWISE_NOT => RegularInstruction::BitwiseNot,

                            // equality
                            InstructionCode::STRUCTURAL_EQUAL => RegularInstruction::StructuralEqual,
                            InstructionCode::EQUAL => RegularInstruction::Equal,
                            InstructionCode::NOT_STRUCTURAL_EQUAL => RegularInstruction::NotStructuralEqual,
                            InstructionCode::NOT_EQUAL => RegularInstruction::NotEqual,
                            InstructionCode::IS => RegularInstruction::Is,
                            InstructionCode::MATCHES => RegularInstruction::Matches,
                            InstructionCode::CREATE_REF => RegularInstruction::CreateRef,
                            InstructionCode::CREATE_REF_MUT => RegularInstruction::CreateRefMut,

                            // slots
                            InstructionCode::ALLOCATE_SLOT => {
                                let address = SlotAddress::read(&mut reader);
                                RegularInstruction::AllocateSlot(yield_unwrap!(address))
                            }
                            InstructionCode::GET_SLOT => {
                                let address = SlotAddress::read(&mut reader);
                                RegularInstruction::GetSlot(yield_unwrap!(address))
                            }
                            InstructionCode::DROP_SLOT => {
                                let address = SlotAddress::read(&mut reader);
                                RegularInstruction::DropSlot(yield_unwrap!(address))
                            }
                            InstructionCode::SET_SLOT => {
                                let address = SlotAddress::read(&mut reader);
                                RegularInstruction::SetSlot(yield_unwrap!(address))
                            }

                            InstructionCode::GET_REF => {
                                let address = RawFullPointerAddress::read(&mut reader);
                                RegularInstruction::GetRef(yield_unwrap!(address))
                            }

                            InstructionCode::GET_LOCAL_REF => {
                                let address = RawLocalPointerAddress::read(&mut reader);
                                RegularInstruction::GetLocalRef(yield_unwrap!(address))
                            }

                            InstructionCode::GET_INTERNAL_REF => {
                                let address =
                                    RawInternalPointerAddress::read(&mut reader);
                                RegularInstruction::GetInternalRef(yield_unwrap!(address))
                            }

                            InstructionCode::ADD_ASSIGN => {
                                let address = SlotAddress::read(&mut reader);
                                RegularInstruction::AddAssign(yield_unwrap!(address))
                            }

                            InstructionCode::SUBTRACT_ASSIGN => {
                                let address = SlotAddress::read(&mut reader);
                                RegularInstruction::SubtractAssign(yield_unwrap!(address))
                            }

                            InstructionCode::TYPED_VALUE => {
                                next_instructions_stack.push_next_regular(1);
                                next_instructions_stack.push_next_type(1);
                                continue;
                            }
                            InstructionCode::TYPE_EXPRESSION => {
                                next_instructions_stack.push_next_regular(1);
                                next_instructions_stack.push_next_type(1);
                                continue;
                            }

                            _ => return yield Err(DXBParserError::InvalidBinaryCode(
                                instruction_code as u8,
                            )),
                        }
                    }.into(),

                    NextInstructionType::Type => {
                        let instruction_code = yield_unwrap!(get_next_type_instruction_code(&mut reader));
                        match instruction_code {
                            TypeInstructionCode::TYPE_LIST_START => {
                                TypeInstruction::ListStart
                            }
                            TypeInstructionCode::TYPE_LITERAL_INTEGER => {
                                let integer_data = IntegerData::read(&mut reader);
                                TypeInstruction::LiteralInteger(yield_unwrap!(integer_data))
                            }
                            TypeInstructionCode::TYPE_WITH_IMPLS => {
                                let impl_data = ImplTypeData::read(&mut reader);
                                next_instructions_stack.push_next_type(1);
                                TypeInstruction::ImplType(yield_unwrap!(impl_data))
                            }
                            TypeInstructionCode::TYPE_REFERENCE => {
                                let ref_data = TypeReferenceData::read(&mut reader);
                                TypeInstruction::TypeReference(yield_unwrap!(ref_data))
                            }
                            _ => core::todo!("#426 Undescribed by author."),
                        }
                    }.into()
                });
            }
        },
    )
}

fn get_next_regular_instruction_code(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<InstructionCode, DXBParserError> {
    let instruction_code = u8::read(&mut reader)
        .map_err(|err| DXBParserError::FailedToReadInstructionCode)?;

    InstructionCode::try_from(instruction_code)
        .map_err(|_| DXBParserError::InvalidInstructionCode(instruction_code))
}

fn get_next_type_instruction_code(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<TypeInstructionCode, DXBParserError> {
    let instruction_code = u8::read(&mut reader)
        .map_err(|err| DXBParserError::FailedToReadInstructionCode)?;

    TypeInstructionCode::try_from(instruction_code)
        .map_err(|_| DXBParserError::InvalidInstructionCode(instruction_code))
}