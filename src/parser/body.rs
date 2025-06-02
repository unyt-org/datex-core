use std::fmt::Display;
use std::io::Cursor;
use binrw::BinRead;
use log::info;
use crate::stdlib::cell::Cell;
use crate::stdlib::fmt;

use crate::datex_values_old::{
    SlotIdentifier, Type,
};
use crate::decompiler::ScopeType;
use crate::global::binary_codes::InstructionCode;
use crate::global::protocol_structures::instructions::{Float64Data, Instruction, Int16Data, Int32Data, Int64Data, Int8Data, ShortTextData, ShortTextDataRaw, TextData, TextDataRaw};
use crate::utils::buffers;

fn extract_slot_identifier(
    dxb_body: &[u8],
    index: &mut usize,
) -> SlotIdentifier {
    let length = buffers::read_u8(dxb_body, index);
    // binary name (2 byte number) TODO: length no longer required
    if length == 0 {
        let index = buffers::read_u16(dxb_body, index);
        SlotIdentifier::new(index)
    }
    // string name TODO: deprecated
    else {
        let _name = buffers::read_string_utf8(dxb_body, index, length as usize);
        SlotIdentifier::default()
    }
}

fn extract_scope(dxb_body: &[u8], index: &mut usize) -> Vec<u8> {
    let size = buffers::read_u32(dxb_body, index);
    buffers::read_vec_slice(dxb_body, index, size as usize)
}

fn extract_type<'a>(
    dxb_body: &'a [u8],
    index: &'a mut usize,
    is_extended: bool,
) -> Type {
    let namespace_length = buffers::read_u8(dxb_body, index);
    let name_length = buffers::read_u8(dxb_body, index);
    let mut variation_length = 0;
    let mut _has_parameters = false; // TODO:get params

    if is_extended {
        variation_length = buffers::read_u8(dxb_body, index);
        _has_parameters = buffers::read_u8(dxb_body, index) != 0;
    }

    let namespace =
        buffers::read_string_utf8(dxb_body, index, namespace_length as usize);
    let name = buffers::read_string_utf8(dxb_body, index, name_length as usize);
    let mut variation: Option<String> = None;

    if is_extended && variation_length != 0 {
        variation = Some(buffers::read_string_utf8(
            dxb_body,
            index,
            variation_length as usize,
        ));
    };

    Type {
        namespace,
        name,
        variation,
    }
}


#[derive(Debug)]
pub enum ParserError {
    InvalidBinaryCode(u8),
    FailedToReadInstructionCode,
    FmtError(fmt::Error),
    BinRwError(binrw::Error),
    FromUtf8Error(std::string::FromUtf8Error),
    InvalidScopeEndType{expected: ScopeType, found: ScopeType}
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
            ParserError::InvalidBinaryCode(code) => write!(f, "Invalid binary code: {code}"),
            ParserError::FailedToReadInstructionCode => write!(f, "Failed to read instruction code"),
            ParserError::FmtError(err) => write!(f, "Formatting error: {err}"),
            ParserError::BinRwError(err) => write!(f, "Binary read/write error: {err}"),
            ParserError::FromUtf8Error(err) => write!(f, "UTF-8 conversion error: {err}"),
            ParserError::InvalidScopeEndType { expected, found } => {
                write!(f, "Invalid scope end type: expected {expected:?}, found {found:?}")
            }
        }
    }
}


fn get_short_text_data(mut reader: &mut Cursor<&[u8]>) -> Result<ShortTextData, ParserError> {
    let raw_data = ShortTextDataRaw::read(&mut reader);
    if let Err(err) = raw_data {
        Err(err.into())
    } else {
        let raw_data = raw_data?;
        let text = String::from_utf8(raw_data.text);
        if let Err(err) = text {
            Err(err.into())
        }
        else {
            let text = text?;
            Ok(ShortTextData(text))
        }
    }
}

fn get_text_data(mut reader: &mut Cursor<&[u8]>) -> Result<TextData, ParserError> {
    let raw_data = TextDataRaw::read(&mut reader);
    if let Err(err) = raw_data {
        Err(err.into())
    } else {
        let raw_data = raw_data?;
        let text = String::from_utf8(raw_data.text);
        if let Err(err) = text {
            Err(err.into())
        }
        else {
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
                    info!("End of dxb_body reached.");
                    return;
                }
                
                let instruction_code = u8::read(&mut reader);
                if let Err(err) = instruction_code {
                    yield Err(err.into());
                    return;
                }

                let instruction_code = InstructionCode::try_from(instruction_code.unwrap());
                if instruction_code.is_err() {
                    yield Err(ParserError::FailedToReadInstructionCode);
                    return;
                }
                let instruction_code = instruction_code.unwrap();
                info!("Instruction code: {:?}", instruction_code);

                yield match instruction_code {
                    InstructionCode::INT_8 => {
                        let data = Int8Data::read(&mut reader);
                        if let Err(err) = data { Err(err.into()) }
                        else { Ok(Instruction::Int8(data.unwrap())) }
                    }
                    
                    InstructionCode::INT_16 => {
                        let data = Int16Data::read(&mut reader);
                        if let Err(err) = data { Err(err.into()) }
                        else { Ok(Instruction::Int16(data.unwrap())) }
                    }

                    InstructionCode::INT_32 => {
                        let data = Int32Data::read(&mut reader);
                        if let Err(err) = data { Err(err.into()) }
                        else { Ok(Instruction::Int32(data.unwrap())) }
                    }
                    
                    InstructionCode::INT_64 => {
                        let data = Int64Data::read(&mut reader);
                        if let Err(err) = data { Err(err.into()) }
                        else { Ok(Instruction::Int64(data.unwrap())) }
                    }
                    
                    InstructionCode::FLOAT_64 => {
                        let data = Float64Data::read(&mut reader);
                        if let Err(err) = data { Err(err.into()) }
                        else { Ok(Instruction::Float64(data.unwrap())) }
                    }
                    
                    InstructionCode::SHORT_TEXT => {
                        let short_text_data = get_short_text_data(&mut reader);
                        if let Err(err) = short_text_data {
                            Err(err)
                        } else {
                            Ok(Instruction::ShortText(short_text_data.unwrap()))
                        }
                    }
                    
                    InstructionCode::TEXT => {
                        let text_data = get_text_data(&mut reader);
                        if let Err(err) = text_data {
                            Err(err)
                        } else {
                            Ok(Instruction::Text(text_data.unwrap()))
                        }
                    }
                    
                    InstructionCode::TRUE => {
                        Ok(Instruction::True)
                    }
                    InstructionCode::FALSE => {
                        Ok(Instruction::False)
                    }
                    InstructionCode::NULL => {
                        Ok(Instruction::Null)
                    }
                    
                    // complex terms
                    InstructionCode::ARRAY_START => {
                        Ok(Instruction::ArrayStart)
                    }
                    InstructionCode::OBJECT_START => {
                        Ok(Instruction::ObjectStart)
                    }
                    InstructionCode::TUPLE_START => {
                        Ok(Instruction::TupleStart)
                    }
                    InstructionCode::SCOPE_START => {
                        Ok(Instruction::ScopeStart)
                    }
                    InstructionCode::ARRAY_END => {
                        Ok(Instruction::ArrayEnd)
                    }
                    InstructionCode::OBJECT_END => {
                        Ok(Instruction::ObjectEnd)
                    }
                    InstructionCode::TUPLE_END => {
                        Ok(Instruction::TupleEnd)
                    }
                    InstructionCode::SCOPE_END => {
                        Ok(Instruction::ScopeEnd)
                    }
                    
                    InstructionCode::KEY_VALUE_SHORT_TEXT => {
                        let short_text_data = get_short_text_data(&mut reader);
                        if let Err(err) = short_text_data {
                            Err(err)
                        } else {
                            Ok(Instruction::KeyValueShortText(short_text_data.unwrap()))
                        }
                    }
                    
                    InstructionCode::KEY_VALUE_TEXT => {
                        let text_data = get_text_data(&mut reader);
                        if let Err(err) = text_data {
                            Err(err)
                        } else {
                            Ok(Instruction::KeyValueText(text_data.unwrap()))
                        }
                    }
                    
                    InstructionCode::KEY_VALUE_DYNAMIC => {
                        Ok(Instruction::KeyValueDynamic)
                    }
                    
                    InstructionCode::CLOSE_AND_STORE => {
                        Ok(Instruction::CloseAndStore)
                    }
                    
                    // operations
                    InstructionCode::ADD => {
                        Ok(Instruction::Add)
                    }

                    _ => {
                        Err(ParserError::InvalidBinaryCode(instruction_code as u8))
                    }
                }
            }
        }
    )
}