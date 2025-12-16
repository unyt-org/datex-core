use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::assignment::AssignmentOperator;
use crate::global::protocol_structures::instructions::{
    ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data,
    FloatAsInt32Data, ImplTypeData, Instruction, InstructionBlockData,
    Int8Data, Int16Data, Int32Data, Int64Data, Int128Data, IntegerData,
    ListData, MapData, RawFullPointerAddress, RawInternalPointerAddress,
    RegularInstruction, ShortListData, ShortMapData, ShortStatementsData,
    ShortTextData, ShortTextDataRaw, SlotAddress, TextData, TextDataRaw,
    TypeInstruction, TypeReferenceData, UInt8Data, UInt16Data, UInt32Data,
    UInt64Data, UInt128Data, UnboundedStatementsData,
};
use crate::global::type_instruction_codes::TypeInstructionCode;
use crate::parser::next_instructions_stack::{
    NextInstructionType, NotInUnboundedRegularScopeError,
};
use crate::runtime::execution::macros::yield_unwrap;
use crate::stdlib::convert::TryFrom;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::FromUtf8Error;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::values::core_values::endpoint::Endpoint;
use binrw::BinRead;
use binrw::io::Cursor;
use core::cell::RefCell;
use core::fmt;
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;
use crate::global::protocol_structures::instructions::{
    RawLocalPointerAddress, StatementsData,
};
use crate::parser::next_instructions_stack::NextInstructionsStack;
use log::info;

#[derive(Debug)]
pub enum DXBParserError {
    InvalidEndpoint(String),
    InvalidBinaryCode(u8),
    FailedToReadInstructionCode,
    InvalidInstructionCode(u8),
    /// Returned when the end of the DXB body is reached, but further instructions are expected.
    ExpectingMoreInstructions,
    FmtError(fmt::Error),
    BinRwError(binrw::Error),
    FromUtf8Error(FromUtf8Error),
    NotInUnboundedRegularScopeError,
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
            DXBParserError::ExpectingMoreInstructions => {
                core::write!(f, "Expecting more instructions")
            }
            DXBParserError::NotInUnboundedRegularScopeError => {
                core::write!(f, "Not in unbounded regular scope error")
            }
        }
    }
}

// TODO: we must ensure while an execution for a block runs, no other executions run using the same next_instructions_stack - maybe also find a solution without Rc<RefCell>
pub fn iterate_instructions(
    dxb_body_ref: Rc<RefCell<Vec<u8>>>,
) -> impl Iterator<Item = Result<Instruction, DXBParserError>> {
    gen move {
        // create a stack to track next instructions
        let mut next_instructions_stack = NextInstructionsStack::default();

        // get reader for dxb_body
        let mut dxb_body = core::mem::take(&mut *dxb_body_ref.borrow_mut());
        let mut len = dxb_body.len();
        let mut reader = Cursor::new(dxb_body);

        loop {
            // if cursor is at the end, check if more instructions are expected, else end iteration
            if reader.position() as usize >= len {
                // indicates that more instructions need to be read
                if !next_instructions_stack.is_end() {
                    yield Err(DXBParserError::ExpectingMoreInstructions);
                    // assume that more instructions are loaded into dxb_body externally after this yield
                    // so we just reload the dxb_body from the Rc<RefCell>
                    dxb_body =
                        core::mem::take(&mut *dxb_body_ref.borrow_mut());
                    len = dxb_body.len();
                    reader = Cursor::new(dxb_body);
                    continue;
                }
                return;
            }

            let next_instruction_type = next_instructions_stack.pop();

            // parse instruction based on its type
            let instruction = (match next_instruction_type {
                NextInstructionType::End => return, // end of instructions

                NextInstructionType::Regular => {
                    let instruction_code = yield_unwrap!(
                        get_next_regular_instruction_code(&mut reader)
                    );

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
                            RegularInstruction::BigInteger(yield_unwrap!(
                                data
                            ))
                        }

                        InstructionCode::DECIMAL_F32 => {
                            let data = Float32Data::read(&mut reader);
                            RegularInstruction::DecimalF32(yield_unwrap!(
                                data
                            ))
                        }
                        InstructionCode::DECIMAL_F64 => {
                            let data = Float64Data::read(&mut reader);
                            RegularInstruction::DecimalF64(yield_unwrap!(
                                data
                            ))
                        }
                        InstructionCode::DECIMAL_BIG => {
                            let data = DecimalData::read(&mut reader);
                            RegularInstruction::Decimal(yield_unwrap!(data))
                        }
                        InstructionCode::DECIMAL_AS_INT_16 => {
                            let data = FloatAsInt16Data::read(&mut reader);
                            RegularInstruction::DecimalAsInt16(
                                yield_unwrap!(data),
                            )
                        }
                        InstructionCode::DECIMAL_AS_INT_32 => {
                            let data = FloatAsInt32Data::read(&mut reader);
                            RegularInstruction::DecimalAsInt32(
                                yield_unwrap!(data),
                            )
                        }

                        InstructionCode::REMOTE_EXECUTION => {
                            let data =
                                InstructionBlockData::read(&mut reader);
                            next_instructions_stack.push_next_regular(1); // receivers
                            RegularInstruction::RemoteExecution(
                                yield_unwrap!(data),
                            )
                        }

                        InstructionCode::SHORT_TEXT => {
                            let raw_data =
                                ShortTextDataRaw::read(&mut reader);
                            let text = yield_unwrap!(String::from_utf8(
                                yield_unwrap!(raw_data).text
                            ));
                            RegularInstruction::ShortText(ShortTextData(
                                text,
                            ))
                        }

                        InstructionCode::ENDPOINT => {
                            let endpoint_data = Endpoint::read(&mut reader);
                            RegularInstruction::Endpoint(yield_unwrap!(
                                endpoint_data
                            ))
                        }

                        InstructionCode::TEXT => {
                            let raw_data = TextDataRaw::read(&mut reader);
                            let text = yield_unwrap!(String::from_utf8(
                                yield_unwrap!(raw_data).text
                            ));
                            RegularInstruction::Text(TextData(text))
                        }

                        InstructionCode::TRUE => RegularInstruction::True,
                        InstructionCode::FALSE => RegularInstruction::False,
                        InstructionCode::NULL => RegularInstruction::Null,

                        // collections
                        InstructionCode::LIST => {
                            let list_data =
                                yield_unwrap!(ListData::read(&mut reader));
                            next_instructions_stack
                                .push_next_regular(list_data.element_count);
                            RegularInstruction::List(list_data)
                        }
                        InstructionCode::SHORT_LIST => {
                            let list_data = yield_unwrap!(
                                ShortListData::read(&mut reader)
                            );
                            next_instructions_stack.push_next_regular(
                                list_data.element_count as u32,
                            );
                            RegularInstruction::ShortList(ListData {
                                element_count: list_data.element_count
                                    as u32,
                            })
                        }
                        InstructionCode::MAP => {
                            let map_data =
                                yield_unwrap!(MapData::read(&mut reader));
                            next_instructions_stack
                                .push_next_regular(map_data.element_count);
                            RegularInstruction::Map(map_data)
                        }
                        InstructionCode::SHORT_MAP => {
                            let map_data = yield_unwrap!(
                                ShortMapData::read(&mut reader)
                            );
                            next_instructions_stack.push_next_regular(
                                map_data.element_count as u32,
                            );
                            RegularInstruction::ShortMap(MapData {
                                element_count: map_data.element_count
                                    as u32,
                            })
                        }

                        InstructionCode::STATEMENTS => {
                            let statements_data = yield_unwrap!(
                                StatementsData::read(&mut reader)
                            );
                            next_instructions_stack.push_next_regular(
                                statements_data.statements_count,
                            );
                            RegularInstruction::Statements(statements_data)
                        }
                        InstructionCode::SHORT_STATEMENTS => {
                            let statements_data = yield_unwrap!(
                                ShortStatementsData::read(&mut reader)
                            );
                            next_instructions_stack.push_next_regular(
                                statements_data.statements_count as u32,
                            );
                            // convert ShortStatementsData to StatementsData for simplicity
                            RegularInstruction::ShortStatements(
                                StatementsData {
                                    statements_count: statements_data
                                        .statements_count
                                        as u32,
                                    terminated: statements_data.terminated,
                                },
                            )
                        }

                        InstructionCode::UNBOUNDED_STATEMENTS => {
                            next_instructions_stack
                                .push_next_regular_unbounded();
                            RegularInstruction::UnboundedStatements
                        }

                        InstructionCode::UNBOUNDED_STATEMENTS_END => {
                            let statements_data = yield_unwrap!(
                                UnboundedStatementsData::read(&mut reader)
                            );
                            yield_unwrap!(
                                next_instructions_stack
                                    .pop_unbounded_regular()
                            );
                            RegularInstruction::UnboundedStatementsEnd(
                                statements_data.terminated,
                            )
                        }

                        InstructionCode::APPLY_ZERO => {
                            RegularInstruction::Apply(ApplyData {
                                arg_count: 0,
                            })
                        }
                        InstructionCode::APPLY_SINGLE => {
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::Apply(ApplyData {
                                arg_count: 1,
                            })
                        }

                        InstructionCode::APPLY => {
                            let apply_data =
                                yield_unwrap!(ApplyData::read(&mut reader));
                            next_instructions_stack.push_next_regular(
                                apply_data.arg_count as u32,
                            ); // each argument is at least one instruction
                            RegularInstruction::Apply(apply_data)
                        }

                        InstructionCode::DEREF => {
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::Deref
                        }
                        InstructionCode::ASSIGN_TO_REF => {
                            next_instructions_stack.push_next_regular(2);
                            let operator = yield_unwrap!(
                                get_next_regular_instruction_code(
                                    &mut reader
                                )
                            );
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
                            let raw_data =
                                ShortTextDataRaw::read(&mut reader);
                            let text = yield_unwrap!(String::from_utf8(
                                yield_unwrap!(raw_data).text
                            ));
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::KeyValueShortText(
                                ShortTextData(text),
                            )
                        }

                        InstructionCode::KEY_VALUE_DYNAMIC => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::KeyValueDynamic
                        }

                        // operations
                        InstructionCode::ADD => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::Add
                        }
                        InstructionCode::SUBTRACT => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::Subtract
                        }
                        InstructionCode::MULTIPLY => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::Multiply
                        }
                        InstructionCode::DIVIDE => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::Divide
                        }

                        InstructionCode::UNARY_MINUS => {
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::UnaryMinus
                        }
                        InstructionCode::UNARY_PLUS => {
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::UnaryPlus
                        }
                        InstructionCode::BITWISE_NOT => {
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::BitwiseNot
                        }

                        // equality
                        InstructionCode::STRUCTURAL_EQUAL => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::StructuralEqual
                        }
                        InstructionCode::EQUAL => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::Equal
                        }
                        InstructionCode::NOT_STRUCTURAL_EQUAL => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::NotStructuralEqual
                        }
                        InstructionCode::NOT_EQUAL => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::NotEqual
                        }
                        InstructionCode::IS => {
                            next_instructions_stack.push_next_regular(2);
                            RegularInstruction::Is
                        }
                        InstructionCode::MATCHES => {
                            next_instructions_stack.push_next_type(1); // type to match against
                            next_instructions_stack.push_next_regular(1); // value to check
                            RegularInstruction::Matches
                        }
                        InstructionCode::CREATE_REF => {
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::CreateRef
                        }
                        InstructionCode::CREATE_REF_MUT => {
                            next_instructions_stack.push_next_regular(1);
                            RegularInstruction::CreateRefMut
                        }

                        // slots
                        InstructionCode::ALLOCATE_SLOT => {
                            next_instructions_stack.push_next_regular(1);
                            let address = SlotAddress::read(&mut reader);
                            RegularInstruction::AllocateSlot(yield_unwrap!(
                                address
                            ))
                        }
                        InstructionCode::GET_SLOT => {
                            let address = SlotAddress::read(&mut reader);
                            RegularInstruction::GetSlot(yield_unwrap!(
                                address
                            ))
                        }
                        InstructionCode::DROP_SLOT => {
                            let address = SlotAddress::read(&mut reader);
                            RegularInstruction::DropSlot(yield_unwrap!(
                                address
                            ))
                        }
                        InstructionCode::SET_SLOT => {
                            next_instructions_stack.push_next_regular(1);
                            let address = SlotAddress::read(&mut reader);
                            RegularInstruction::SetSlot(yield_unwrap!(
                                address
                            ))
                        }

                        InstructionCode::GET_REF => {
                            let address =
                                RawFullPointerAddress::read(&mut reader);
                            RegularInstruction::GetRef(yield_unwrap!(
                                address
                            ))
                        }

                        InstructionCode::GET_LOCAL_REF => {
                            let address =
                                RawLocalPointerAddress::read(&mut reader);
                            RegularInstruction::GetLocalRef(yield_unwrap!(
                                address
                            ))
                        }

                        InstructionCode::GET_INTERNAL_REF => {
                            let address = RawInternalPointerAddress::read(
                                &mut reader,
                            );
                            RegularInstruction::GetInternalRef(
                                yield_unwrap!(address),
                            )
                        }

                        InstructionCode::ADD_ASSIGN => {
                            let address = SlotAddress::read(&mut reader);
                            RegularInstruction::AddAssign(yield_unwrap!(
                                address
                            ))
                        }

                        InstructionCode::SUBTRACT_ASSIGN => {
                            let address = SlotAddress::read(&mut reader);
                            RegularInstruction::SubtractAssign(
                                yield_unwrap!(address),
                            )
                        }

                        InstructionCode::TYPED_VALUE => {
                            next_instructions_stack.push_next_regular(1);
                            next_instructions_stack.push_next_type(1);
                            RegularInstruction::TypedValue
                        }
                        InstructionCode::TYPE_EXPRESSION => {
                            next_instructions_stack.push_next_type(1);
                            RegularInstruction::TypeExpression
                        }

                        _ => {
                            return yield Err(
                                DXBParserError::InvalidBinaryCode(
                                    instruction_code as u8,
                                ),
                            );
                        }
                    }
                }
                .into(),

                NextInstructionType::Type => {
                    let instruction_code = yield_unwrap!(
                        get_next_type_instruction_code(&mut reader)
                    );
                    match instruction_code {
                        TypeInstructionCode::TYPE_LIST => {
                            let list_data =
                                yield_unwrap!(ListData::read(&mut reader));
                            next_instructions_stack
                                .push_next_regular(list_data.element_count);
                            TypeInstruction::List(list_data)
                        }
                        TypeInstructionCode::TYPE_LITERAL_INTEGER => {
                            let integer_data =
                                IntegerData::read(&mut reader);
                            TypeInstruction::LiteralInteger(yield_unwrap!(
                                integer_data
                            ))
                        }
                        TypeInstructionCode::TYPE_WITH_IMPLS => {
                            let impl_data = ImplTypeData::read(&mut reader);
                            next_instructions_stack.push_next_type(1);
                            TypeInstruction::ImplType(yield_unwrap!(
                                impl_data
                            ))
                        }
                        TypeInstructionCode::TYPE_REFERENCE => {
                            let ref_data =
                                TypeReferenceData::read(&mut reader);
                            TypeInstruction::TypeReference(yield_unwrap!(
                                ref_data
                            ))
                        }
                        _ => {
                            return yield Err(
                                DXBParserError::InvalidBinaryCode(
                                    instruction_code as u8,
                                ),
                            );
                        }
                    }
                }
                .into(),
            });

            yield Ok(instruction);
        }
    }

}

fn get_next_regular_instruction_code(
    mut reader: &mut Cursor<Vec<u8>>,
) -> Result<InstructionCode, DXBParserError> {
    let instruction_code = u8::read(&mut reader)
        .map_err(|_| DXBParserError::FailedToReadInstructionCode)?;

    InstructionCode::try_from(instruction_code)
        .map_err(|_| DXBParserError::InvalidInstructionCode(instruction_code))
}

fn get_next_type_instruction_code(
    mut reader: &mut Cursor<Vec<u8>>,
) -> Result<TypeInstructionCode, DXBParserError> {
    let instruction_code = u8::read(&mut reader)
        .map_err(|_| DXBParserError::FailedToReadInstructionCode)?;

    TypeInstructionCode::try_from(instruction_code)
        .map_err(|_| DXBParserError::InvalidInstructionCode(instruction_code))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iterate_dxb(
        data: Vec<u8>,
    ) -> impl Iterator<Item = Result<Instruction, DXBParserError>> {
        iterate_instructions(Rc::new(RefCell::new(data)))
    }

    #[test]
    fn invalid_instruction_code() {
        let data = vec![0xFF]; // Invalid instruction code
        let mut iterator = iterate_dxb(data);
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Err(DXBParserError::InvalidInstructionCode(0xFF))
        ));
    }

    #[test]
    fn empty_expect_more_instructions() {
        let data = vec![]; // Empty data
        let mut iterator = iterate_dxb(data);
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Err(DXBParserError::ExpectingMoreInstructions)
        ));
    }

    #[test]
    fn valid_uint8_instruction() {
        let data = vec![InstructionCode::UINT_8 as u8, 42];
        let mut iterator = iterate_dxb(data);
        let result = iterator.next().unwrap();
        match result {
            Ok(Instruction::RegularInstruction(RegularInstruction::UInt8(
                value,
            ))) => {
                assert_eq!(value.0, 42);
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
        let result = iterator.next().unwrap();
        match result {
            Ok(Instruction::RegularInstruction(
                RegularInstruction::ShortText(value),
            )) => {
                assert_eq!(value.0, "Hello");
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
        assert!(matches!(
            iterator.next().unwrap(),
            Ok(Instruction::RegularInstruction(RegularInstruction::Add))
        ));
        // next instruction should be first UINT_8
        assert!(matches!(
            iterator.next().unwrap(),
            Ok(Instruction::RegularInstruction(RegularInstruction::UInt8(
                UInt8Data(10)
            )))
        ));
        // next instruction should be second UINT_8
        assert!(matches!(
            iterator.next().unwrap(),
            Ok(Instruction::RegularInstruction(RegularInstruction::UInt8(
                UInt8Data(20)
            )))
        ));
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
        let mut iterator = iterate_instructions(data_ref.clone());
        // first instruction should be LIST
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Ok(Instruction::RegularInstruction(RegularInstruction::List(_)))
        ));
        // next instruction should error expecting more instructions
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Err(DXBParserError::ExpectingMoreInstructions)
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
        assert!(matches!(
            result,
            Ok(Instruction::RegularInstruction(RegularInstruction::UInt8(
                _
            )))
        ));
        // next instruction should be second UINT_8
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Ok(Instruction::RegularInstruction(RegularInstruction::UInt8(
                _
            )))
        ));
        // ensure no more instructions
        assert!(iterator.next().is_none());
    }

    #[test]
    fn unbounded_expect_more_instructions() {
        let data = vec![InstructionCode::UNBOUNDED_STATEMENTS as u8]; // Start unbounded statements
        let data_ref = Rc::new(RefCell::new(data));
        let mut iterator = iterate_instructions(data_ref.clone());
        // first instruction should be UNBOUNDED_STATEMENTS
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Ok(Instruction::RegularInstruction(
                RegularInstruction::UnboundedStatements
            ))
        ));
        // next instruction should error expecting more instructions
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Err(DXBParserError::ExpectingMoreInstructions)
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
        assert!(matches!(
            result,
            Ok(Instruction::RegularInstruction(RegularInstruction::UInt8(
                _
            )))
        ));
        // next instruction should be UNBOUNDED_STATEMENTS_END
        let result = iterator.next().unwrap();
        assert!(matches!(
            result,
            Ok(Instruction::RegularInstruction(
                RegularInstruction::UnboundedStatementsEnd(_)
            ))
        ));
        // ensure no more instructions
        assert!(iterator.next().is_none());
    }
}
