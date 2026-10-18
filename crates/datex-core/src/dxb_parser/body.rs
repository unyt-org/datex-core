use crate::dxb_parser::next_instructions_stack::{
    NextInstructionType, NextInstructionsStack, NotInUnboundedRegularScopeError,
};

use crate::{
    instruction::{
        Instruction, NestedInstructionResolutionStrategy,
        regular_instruction::RegularInstruction,
        type_instruction::TypeInstruction,
    },
    libs::core::core_lib_id::CoreLibIdIndex,
    prelude::*,
};
use alloc::string::FromUtf8Error;
use binrw::{BinRead, io::Cursor};
use core::{
    cell::RefCell,
    fmt,
    fmt::Display,
    ops::{Deref, Range},
    result::Result,
};

// This is needed to place correct Offsets for Jumps in Conditions
pub type SeekRequest = Rc<RefCell<Option<i32>>>;

#[derive(Debug)]
pub enum DXBParserError {
    InvalidEndpoint(String),
    InvalidBinaryCode(u8),
    FailedToReadInstructionCode,
    InvalidInstructionCode(u8),
    /// Returned when the end of the DXB body is reached, but further instructions are expected.
    ExpectingMoreInstructions(NextInstructionsStack),
    UnexpectedBytesAfterEndOfInstructions,
    FmtError(fmt::Error),
    BinRwError(binrw::Error),
    FromUtf8Error(FromUtf8Error),
    NotInUnboundedRegularScopeError,
    InvalidCoreLibId(CoreLibIdIndex),
}

// custom impl required because binrw::Error does not implement PartialEq
impl PartialEq for DXBParserError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                DXBParserError::InvalidEndpoint(a),
                DXBParserError::InvalidEndpoint(b),
            ) => a == b,
            (
                DXBParserError::InvalidBinaryCode(a),
                DXBParserError::InvalidBinaryCode(b),
            ) => a == b,
            (
                DXBParserError::FailedToReadInstructionCode,
                DXBParserError::FailedToReadInstructionCode,
            ) => true,
            (
                DXBParserError::InvalidInstructionCode(a),
                DXBParserError::InvalidInstructionCode(b),
            ) => a == b,
            (
                DXBParserError::ExpectingMoreInstructions(a),
                DXBParserError::ExpectingMoreInstructions(b),
            ) => a == b,
            (
                DXBParserError::UnexpectedBytesAfterEndOfInstructions,
                DXBParserError::UnexpectedBytesAfterEndOfInstructions,
            ) => true,
            (DXBParserError::FmtError(a), DXBParserError::FmtError(b)) => {
                a.to_string() == b.to_string()
            }
            (DXBParserError::BinRwError(a), DXBParserError::BinRwError(b)) => {
                a.to_string() == b.to_string()
            }
            (
                DXBParserError::FromUtf8Error(a),
                DXBParserError::FromUtf8Error(b),
            ) => a.to_string() == b.to_string(),
            (
                DXBParserError::NotInUnboundedRegularScopeError,
                DXBParserError::NotInUnboundedRegularScopeError,
            ) => true,
            _ => false,
        }
    }
}

impl Eq for DXBParserError {}

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

impl From<NotInUnboundedRegularScopeError> for DXBParserError {
    fn from(_: NotInUnboundedRegularScopeError) -> Self {
        DXBParserError::NotInUnboundedRegularScopeError
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
                core::write!(
                    f,
                    "Encountered an invalid instruction code: {:2X}",
                    code
                )
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
            DXBParserError::ExpectingMoreInstructions(stack) => {
                core::write!(f, "Expecting more instructions: {stack}")
            }
            DXBParserError::UnexpectedBytesAfterEndOfInstructions => {
                core::write!(f, "Unexpected bytes after end of instructions")
            }
            DXBParserError::NotInUnboundedRegularScopeError => {
                core::write!(f, "Not in unbounded regular scope error")
            }
            DXBParserError::InvalidCoreLibId(id) => {
                core::write!(f, "Invalid Core Lib Id: {}", id.0)
            }
        }
    }
}

/// If the "disassembler" feature is enabled, this struct includes a `span` field
/// that represents the range of bytes in the DXB body that correspond to this instruction.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "disassembler", derive(serde::Serialize))]
pub struct InstructionWithSpan {
    pub instruction: Instruction,
    #[cfg(feature = "disassembler")]
    pub span: Range<usize>,
}

impl PartialEq for InstructionWithSpan {
    fn eq(&self, other: &Self) -> bool {
        // always ignore the span when comparing for equality, as it is only relevant for debugging
        self.instruction == other.instruction
    }
}

impl Deref for InstructionWithSpan {
    type Target = Instruction;

    fn deref(&self) -> &Self::Target {
        &self.instruction
    }
}

impl From<Instruction> for InstructionWithSpan {
    fn from(value: Instruction) -> Self {
        InstructionWithSpan {
            instruction: value,
            #[cfg(feature = "disassembler")]
            span: 0..0, // default span, can be updated later
        }
    }
}

impl From<InstructionWithSpan> for Instruction {
    fn from(value: InstructionWithSpan) -> Self {
        value.instruction
    }
}

pub fn iterate_instructions(
    dxb_body_ref: Rc<RefCell<Vec<u8>>>,
    nested_instruction_resolution_strategy: NestedInstructionResolutionStrategy,
) -> impl Iterator<Item = Result<InstructionWithSpan, DXBParserError>> {
    iterate_instructions_with_seek(
        dxb_body_ref,
        nested_instruction_resolution_strategy,
        None,
    )
}

pub gen fn iterate_instructions_with_seek(
    dxb_body_ref: Rc<RefCell<Vec<u8>>>,
    nested_instruction_resolution_strategy: NestedInstructionResolutionStrategy,
    seek_request: Option<Rc<RefCell<Option<i32>>>>,
) -> Result<InstructionWithSpan, DXBParserError> {
    let mut next_instructions_stack = NextInstructionsStack::default();

    let mut dxb_body = core::mem::take(&mut *dxb_body_ref.borrow_mut());
    let mut len = dxb_body.len();
    let mut reader = Cursor::new(dxb_body);

    loop {
        // check for pending seek request
        if let Some(seek_request) = seek_request.as_ref()
            && let Some(seek_offset) = seek_request.borrow_mut().take()
        {
            let new_pos = reader.position() as i64 + seek_offset as i64;
            reader.set_position(new_pos.max(0) as u64);
            continue;
        }

        if reader.position() as usize >= len {
            if !next_instructions_stack.is_end() {
                yield Err(DXBParserError::ExpectingMoreInstructions(
                    next_instructions_stack.clone(),
                ));

                dxb_body = core::mem::take(&mut *dxb_body_ref.borrow_mut());
                len = dxb_body.len();
                reader = Cursor::new(dxb_body);

                continue;
            }

            return;
        }

        let previous_position = reader.position() as usize;

        let next_instruction_type = next_instructions_stack.pop();

        let instruction_result: Result<Instruction, DXBParserError> = try {
            match next_instruction_type {
                NextInstructionType::End => {
                    if len > reader.position() as usize {
                        yield Err(
                            DXBParserError::
                                UnexpectedBytesAfterEndOfInstructions,
                        );
                    }

                    return;
                }

                NextInstructionType::Regular => {
                    let instruction = RegularInstruction::read(&mut reader)
                        .map_err(DXBParserError::BinRwError)?;

                    let instruction = cfg_select! {
                        feature = "disassembler" => instruction
                            .convert_to_nested(
                                nested_instruction_resolution_strategy,
                            )?,
                        _ => instruction,
                    };

                    next_instructions_stack
                        .handle_next_expected_instructions(
                            instruction.get_next_expected_instructions(),
                        )
                        .map_err(|_| {
                            DXBParserError::NotInUnboundedRegularScopeError
                        })?;

                    instruction.into()
                }

                NextInstructionType::Type => {
                    let instruction = TypeInstruction::read(&mut reader)
                        .map_err(DXBParserError::BinRwError)?;

                    next_instructions_stack
                        .handle_next_expected_instructions(
                            instruction.get_next_expected_instructions(),
                        )
                        .map_err(|_| {
                            DXBParserError::NotInUnboundedRegularScopeError
                        })?;

                    instruction.into()
                }
            }
        };

        let instruction = match instruction_result {
            Ok(instruction) => InstructionWithSpan {
                instruction,
                #[cfg(feature = "disassembler")]
                span: (previous_position)..(reader.position() as usize),
            },
            Err(error) => return yield Err(error),
        };

        yield Ok(instruction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::instruction_codes::InstructionCode;
    use core::assert_matches;

    fn iterate_dxb(
        data: Vec<u8>,
    ) -> impl Iterator<Item = Result<Instruction, DXBParserError>> {
        iterate_instructions(
            Rc::new(RefCell::new(data)),
            NestedInstructionResolutionStrategy::default(),
        )
        .map(|instruction_with_span| instruction_with_span.map(|i| i.into()))
    }

    #[test]
    fn invalid_instruction_code() {
        let data = vec![0xFF]; // Invalid instruction code
        let mut iterator = iterate_dxb(data);
        let result = iterator.next().unwrap();
        assert_matches!(result, Err(DXBParserError::BinRwError(_)));
    }

    #[test]
    fn empty_expect_more_instructions() {
        let data = vec![]; // Empty data
        let mut iterator = iterate_dxb(data);
        let result = iterator.next().unwrap();
        assert_matches!(
            result,
            Err(DXBParserError::ExpectingMoreInstructions(_))
        );
    }

    #[test]
    fn valid_uint8_instruction() {
        let data = vec![InstructionCode::UINT_8 as u8, 42];
        let mut iterator = iterate_dxb(data);
        let result = iterator.next().unwrap();
        match result.expect("Expected a valid instruction") {
            Instruction::Regular(instr) => {
                assert_eq!(instr, RegularInstruction::uint8(42));
            }
            _ => panic!("Expected UINT_8 instruction"),
        }
        // Ensure no more instructions
        assert!(iterator.next().is_none());
    }

    #[test]
    fn valid_short_text_instruction() {
        let text = "Hello";
        let text_bytes = text.as_bytes();
        let mut data =
            vec![InstructionCode::SHORT_TEXT as u8, text_bytes.len() as u8];
        data.extend_from_slice(text_bytes);
        let mut iterator = iterate_dxb(data);
        let result = iterator
            .next()
            .unwrap()
            .expect("Expected a valid instruction");
        match result {
            Instruction::Regular(instr) => {
                assert_eq!(
                    instr,
                    RegularInstruction::short_text("Hello".to_string())
                );
            }
            _ => panic!("Expected SHORT_TEXT instruction"),
        }
        // Ensure no more instructions
        assert!(iterator.next().is_none());
    }

    #[test]
    fn valid_add_instruction() {
        let data = vec![
            InstructionCode::ADD as u8,
            // first operand (UINT_8)
            InstructionCode::UINT_8 as u8,
            10,
            // second operand (UINT_8)
            InstructionCode::UINT_8 as u8,
            20,
        ];
        let mut iterator = iterate_dxb(data);
        // first instruction should be ADD
        assert_eq!(
            iterator.next().unwrap(),
            Ok(Instruction::Regular(RegularInstruction::add()))
        );
        // next instruction should be first UINT_8
        assert_eq!(
            iterator.next().unwrap(),
            Ok(Instruction::Regular(RegularInstruction::uint8(10)))
        );
        // next instruction should be second UINT_8
        assert_eq!(
            iterator.next().unwrap(),
            Ok(Instruction::Regular(RegularInstruction::uint8(20)))
        );
        // ensure no more instructions
        assert!(iterator.next().is_none());
    }

    #[test]
    fn error_for_partial_instruction() {
        let data = vec![InstructionCode::UINT_16 as u8, 0x34]; // Incomplete UINT_16 data
        let mut iterator = iterate_dxb(data);
        let result = iterator.next().unwrap();
        assert!(matches!(result, Err(DXBParserError::BinRwError(_))));
    }

    #[test]
    fn expect_more_instructions_after_partial() {
        let data = vec![InstructionCode::LIST as u8, 0x02, 0x00, 0x00, 0x00]; // LIST with 2 elements but no elements provided
        let data_ref = Rc::new(RefCell::new(data));
        let mut iterator = iterate_instructions(
            data_ref.clone(),
            NestedInstructionResolutionStrategy::default(),
        );
        // first instruction should be LIST
        let result = iterator.next().unwrap();
        assert_eq!(
            result.map(|i| Instruction::from(i)),
            Ok(Instruction::Regular(RegularInstruction::list_default(2)))
        );
        // next instruction should error expecting more instructions
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Err(DXBParserError::ExpectingMoreInstructions(_))
        ));

        // now provide more data for the two elements
        let new_data = vec![
            InstructionCode::UINT_8 as u8, // first element
            10,
            InstructionCode::UINT_8 as u8, // second element
            20,
        ];

        *data_ref.borrow_mut() = new_data;

        // next instruction should be first UINT_8
        let result = iterator.next().unwrap();
        assert_eq!(
            result.map(|i| Instruction::from(i)),
            Ok(Instruction::Regular(RegularInstruction::uint8(10)))
        );
        // next instruction should be second UINT_8
        let result = iterator.next().unwrap();
        assert_eq!(
            result.map(|i| Instruction::from(i)),
            Ok(Instruction::Regular(RegularInstruction::uint8(20)))
        );
        // ensure no more instructions
        assert!(iterator.next().is_none());
    }

    #[test]
    fn unbounded_expect_more_instructions() {
        let data = vec![InstructionCode::UNBOUNDED_STATEMENTS as u8]; // Start unbounded statements
        let data_ref = Rc::new(RefCell::new(data));
        let mut iterator = iterate_instructions(
            data_ref.clone(),
            NestedInstructionResolutionStrategy::default(),
        );
        // first instruction should be UNBOUNDED_STATEMENTS
        let result = iterator.next().unwrap();
        assert_eq!(
            Instruction::from(result.unwrap()),
            Instruction::Regular(RegularInstruction::unbounded_statements())
        );
        // next instruction should error expecting more instructions
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Err(DXBParserError::ExpectingMoreInstructions(_))
        ));

        // now provide more data for the statements
        let new_data = vec![
            InstructionCode::UINT_8 as u8, // first statement
            42,
            InstructionCode::UNBOUNDED_STATEMENTS_END as u8, // end unbounded statements
            0x00,
        ];

        *data_ref.borrow_mut() = new_data;

        // next instruction should be first UINT_8
        let result = iterator.next().unwrap();
        assert_eq!(
            Instruction::from(result.unwrap()),
            Instruction::Regular(RegularInstruction::uint8(42))
        );
        // next instruction should be UNBOUNDED_STATEMENTS_END
        let result = iterator.next().unwrap();
        assert_eq!(
            Instruction::from(result.unwrap()),
            Instruction::Regular(RegularInstruction::unbounded_statements_end(
                false
            ))
        );
        // ensure no more instructions
        assert!(iterator.next().is_none());
    }
}
