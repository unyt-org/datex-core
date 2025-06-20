use crate::datex_values::core_values::endpoint::Endpoint;
use crate::stdlib::fmt;
use binrw::BinRead;
use std::fmt::Display;
use std::io::Cursor;
use crate::decompiler::ScopeType;
use crate::global::binary_codes::InstructionCode;
use crate::global::protocol_structures::instructions::{DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, Instruction, Int128Data, Int16Data, Int32Data, Int64Data, Int8Data, ShortTextData, ShortTextDataRaw, SlotAddress, TextData, TextDataRaw};
use crate::utils::buffers;


fn extract_scope(dxb_body: &[u8], index: &mut usize) -> Vec<u8> {
    let size = buffers::read_u32(dxb_body, index);
    buffers::read_vec_slice(dxb_body, index, size as usize)
}

#[derive(Debug)]
pub enum ParserError {
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

impl From<fmt::Error> for ParserError {
    fn from(error: fmt::Error) -> Self {
        ParserError::FmtError(error)
    }
}

impl From<binrw::Error> for ParserError {
    fn from(error: binrw::Error) -> Self {
        ParserError::BinRwError(error)
    }
}

impl From<std::string::FromUtf8Error> for ParserError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        ParserError::FromUtf8Error(error)
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::InvalidBinaryCode(code) => {
                write!(f, "Invalid binary code: {code}")
            }
            ParserError::InvalidEndpoint(endpoint) => {
                write!(f, "Invalid endpoint: {endpoint}")
            }
            ParserError::FailedToReadInstructionCode => {
                write!(f, "Failed to read instruction code")
            }
            ParserError::FmtError(err) => write!(f, "Formatting error: {err}"),
            ParserError::BinRwError(err) => {
                write!(f, "Binary read/write error: {err}")
            }
            ParserError::FromUtf8Error(err) => {
                write!(f, "UTF-8 conversion error: {err}")
            }
            ParserError::InvalidScopeEndType { expected, found } => {
                write!(f, "Invalid scope end type: expected {expected:?}, found {found:?}")
            }
        }
    }
}

fn get_short_text_data(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<ShortTextData, ParserError> {
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
) -> Result<Endpoint, ParserError> {
    let raw_data = Endpoint::read(&mut reader);
    if let Ok(endpoint) = raw_data {
        Ok(endpoint)
    } else {
        Err(raw_data.err().unwrap().into())
    }
}

fn get_text_data(
    mut reader: &mut Cursor<&[u8]>,
) -> Result<TextData, ParserError> {
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

// TODO: refactor: pass a ParserState struct instead of individual parameters
pub fn iterate_instructions<'a>(
    dxb_body: &'a [u8],
) -> impl Iterator<Item = Result<Instruction, ParserError>> + 'a {
    std::iter::from_coroutine(
        #[coroutine]
        move || {
            // get reader for dxb_body
            let mut reader = Cursor::new(dxb_body);
            loop {
                // if cursor is at the end, break
                if reader.position() as usize >= dxb_body.len() {
                    return;
                }

                let instruction_code = u8::read(&mut reader);
                if let Err(err) = instruction_code {
                    yield Err(err.into());
                    return;
                }

                let instruction_code =
                    InstructionCode::try_from(instruction_code.unwrap());
                if instruction_code.is_err() {
                    yield Err(ParserError::FailedToReadInstructionCode);
                    return;
                }
                let instruction_code = instruction_code.unwrap();
                //info!("Instruction code: {:?}", instruction_code);

                yield match instruction_code {
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
                    InstructionCode::ARRAY_START => Ok(Instruction::ArrayStart),
                    InstructionCode::OBJECT_START => {
                        Ok(Instruction::ObjectStart)
                    }
                    InstructionCode::TUPLE_START => Ok(Instruction::TupleStart),
                    InstructionCode::SCOPE_START => Ok(Instruction::ScopeStart),
                    InstructionCode::SCOPE_END => Ok(Instruction::ScopeEnd),

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

                    _ => Err(ParserError::InvalidBinaryCode(
                        instruction_code as u8,
                    )),
                }
            }
        },
    )
}
