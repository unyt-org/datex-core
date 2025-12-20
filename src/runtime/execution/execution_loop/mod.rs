pub mod interrupts;
mod operations;
pub mod regular_instruction_execution;
pub mod state;
pub mod type_instruction_execution;

use crate::global::protocol_structures::instructions::{DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, Instruction, IntegerData, RawPointerAddress, RegularInstruction, ShortTextData, SlotAddress, TextData, TypeInstruction};
use crate::parser::body::{DXBParserError, iterate_instructions};
use crate::runtime::execution::execution_loop::interrupts::{
    ExecutionInterrupt, ExternalExecutionInterrupt, InterruptProvider,
    InterruptResult,
};
use crate::runtime::execution::execution_loop::regular_instruction_execution::execute_regular_instruction;
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::macros::{interrupt, interrupt_with_maybe_value, interrupt_with_value, next_iter, yield_unwrap};
use crate::runtime::execution::{ExecutionError, InvalidProgramError};
use crate::stdlib::rc::Rc;
use crate::traits::apply::Apply;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;
use crate::global::operators::{BinaryOperator, UnaryOperator};
use crate::parser::instruction_collector::{CollectedResults, CollectionResultsPopper, InstructionCollector};
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::execution_loop::type_instruction_execution::get_next_type;
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::list::List;
use crate::values::core_values::map::{Map, OwnedMapKey};
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;

#[derive(Debug)]
enum CollectedExecutionResult {
    /// contains an optional ValueContainer that is intercepted by the consumer of a value or passed as the final result at the end of execution
    Value(Option<ValueContainer>),
    /// contains a Type that is intercepted by a consumer of a type value
    Type(Type),
    /// contains a key-value pair that is intercepted by a map construction operation
    KeyValuePair((OwnedMapKey, Option<ValueContainer>)),
}

impl From<Option<ValueContainer>> for CollectedExecutionResult {
    fn from(value: Option<ValueContainer>) -> Self {
        CollectedExecutionResult::Value(value)
    }
}
impl From<ValueContainer> for CollectedExecutionResult {
    fn from(value: ValueContainer) -> Self {
        CollectedExecutionResult::Value(Some(value))
    }
}
impl From<Type> for CollectedExecutionResult {
    fn from(value: Type) -> Self {
        CollectedExecutionResult::Type(value)
    }
}
impl From<(OwnedMapKey, Option<ValueContainer>)> for CollectedExecutionResult {
    fn from(value: (OwnedMapKey, Option<ValueContainer>)) -> Self {
        CollectedExecutionResult::KeyValuePair(value)
    }
}


impl CollectionResultsPopper<CollectedExecutionResult, Option<ValueContainer>, OwnedMapKey, Type>
for CollectedResults<CollectedExecutionResult> {
    fn try_extract_value_result(result: CollectedExecutionResult) -> Option<Option<ValueContainer>> {
        match result {
            CollectedExecutionResult::Value(val) => Some(val),
            _ => None
        }
    }

    fn try_extract_type_result(result: CollectedExecutionResult) -> Option<Type> {
        match result {
            CollectedExecutionResult::Type(ty) => Some(ty),
            _ => None
        }
    }

    fn try_extract_key_value_pair_result(result: CollectedExecutionResult) -> Option<(OwnedMapKey, Option<ValueContainer>)> {
        match result {
            CollectedExecutionResult::KeyValuePair((key, value)) => Some((key, value)),
            _ => None
        }
    }
}

impl CollectedResults<CollectedExecutionResult> {
    fn collect_value_results_assert_existing(mut self) -> Result<Vec<ValueContainer>, ExecutionError> {
        let count = self.len();
        let mut expressions = Vec::with_capacity(count);
        for _ in 0..count {
            expressions.push(
                self.pop_value_result()
                    .ok_or(ExecutionError::InvalidProgram(InvalidProgramError::ExpectedValue))?
            );
        }
        expressions.reverse();
        Ok(expressions)
    }

    fn collect_key_value_pair_results_assert_existing(mut self) -> Result<Vec<(OwnedMapKey, ValueContainer)>, ExecutionError> {
        let count = self.len();
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let (key, value) = self.pop_key_value_pair_result();
            pairs.push(
                (
                    key,
                    value.ok_or(ExecutionError::InvalidProgram(InvalidProgramError::ExpectedValue))?
                )
            );
        }
        pairs.reverse();
        Ok(pairs)
    }
}

//
// /// Main execution loop that drives the execution of the DXB body
// /// The interrupt_provider is used to provide results for synchronous or asynchronous I/O operations
// pub fn execution_loop(
//     state: RuntimeExecutionState,
//     dxb_body: Rc<RefCell<Vec<u8>>>,
//     interrupt_provider: InterruptProvider,
// ) -> impl Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>> {
//     gen move {
//         let mut instruction_iterator = iterate_instructions(dxb_body);
//         let mut slots = state.slots;
//
//         let mut active_value: Option<ValueContainer> = None;
//         let mut collector = InstructionCollector::<CollectedExecutionResult>::default();
//
//         for instruction in iterate_instructions(dxb_body) {
//             let instruction = yield_unwrap!(instruction);
//
//             let result = match instruction {
//                 // handle regular instructions
//                 Instruction::RegularInstruction(regular_instruction) => {
//                     let regular_instruction = collector.default_regular_instruction_collection(regular_instruction);
//
//                     let expr: Option<Option<ValueContainer>> = if let Some(regular_instruction) = regular_instruction {
//                         Some(match regular_instruction {
//                             // boolean
//                             RegularInstruction::True => Some(true.into()),
//                             RegularInstruction::False => Some(false.into()),
//
//                             // integers
//                             RegularInstruction::Int8(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::Int16(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::Int32(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::Int64(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::Int128(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//
//                             // unsigned integers
//                             RegularInstruction::UInt8(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::UInt16(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::UInt32(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::UInt64(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//                             RegularInstruction::UInt128(integer) => {
//                                 Some(TypedInteger::from(integer.0).into())
//                             }
//
//                             // big integers
//                             RegularInstruction::BigInteger(IntegerData(integer)) => {
//                                 Some(TypedInteger::Big(integer).into())
//                             }
//
//                             // default integer
//                             RegularInstruction::Integer(IntegerData(i8)) => {
//                                 Some(i8.into())
//                             }
//
//                             // specific floats
//                             RegularInstruction::DecimalF32(Float32Data(f32)) => {
//                                 Some(TypedDecimal::from(f32).into())
//                             }
//                             RegularInstruction::DecimalF64(Float64Data(f64)) => {
//                                 Some(TypedDecimal::from(f64).into())
//                             }
//                             // big decimal
//                             RegularInstruction::BigDecimal(DecimalData(big_decimal)) => {
//                                 Some(TypedDecimal::Decimal(big_decimal).into())
//                             }
//
//                             // default decimals
//                             RegularInstruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
//                                 Some(Decimal::from(i16 as f32).into())
//                             }
//                             RegularInstruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
//                                 Some(Decimal::from(i32 as f32).into())
//                             }
//                             RegularInstruction::Decimal(DecimalData(big_decimal)) => {
//                                 Some(big_decimal.into())
//                             }
//
//                             // endpoint
//                             RegularInstruction::Endpoint(endpoint) => Some(endpoint.into()),
//
//                             // null
//                             RegularInstruction::Null => Some(Value::null().into()),
//
//                             // text
//                             RegularInstruction::ShortText(ShortTextData(text)) => {
//                                 Some(text.into())
//                             }
//                             RegularInstruction::Text(TextData(text)) => Some(text.into()),
//
//                             RegularInstruction::GetRef(address) => Some(interrupt_with_value!(
//                                     interrupt_provider,
//                                     ExecutionInterrupt::External(
//                                         ExternalExecutionInterrupt::ResolvePointer(address)
//                                     )
//                                 )),
//                             RegularInstruction::GetLocalRef(address) => {
//                                 Some(interrupt_with_value!(
//                                     interrupt_provider,
//                                     ExecutionInterrupt::External(
//                                         ExternalExecutionInterrupt::ResolveLocalPointer(
//                                             address
//                                         )
//                                     )
//                                 ))
//                             }
//                             RegularInstruction::GetInternalRef(address) => {
//                                 Some(interrupt_with_value!(
//                                     interrupt_provider,
//                                     ExecutionInterrupt::External(
//                                         ExternalExecutionInterrupt::ResolveInternalPointer(
//                                             address
//                                         )
//                                     )
//                                 ))
//                             }
//
//                             RegularInstruction::GetSlot(SlotAddress(address)) => {
//                                 Some(interrupt_with_value!(
//                                     interrupt_provider,
//                                     ExecutionInterrupt::GetSlotValue(address)
//                                 ))
//                             }
//
//                             // NOTE: make sure that each possible match case is either implemented in the default collection or here
//                             // If an instruction is implemented in the default collection, it should be marked as unreachable!() here
//                             RegularInstruction::Statements(_) |
//                             RegularInstruction::ShortStatements(_) |
//                             RegularInstruction::UnboundedStatements |
//                             RegularInstruction::UnboundedStatementsEnd(_) |
//                             RegularInstruction::List(_) |
//                             RegularInstruction::ShortList(_)  |
//                             RegularInstruction::Map(_) |
//                             RegularInstruction::ShortMap(_) |
//                             RegularInstruction::KeyValueDynamic |
//                             RegularInstruction::KeyValueShortText(_) |
//                             RegularInstruction::Add |
//                             RegularInstruction::Subtract |
//                             RegularInstruction::Multiply |
//                             RegularInstruction::Divide |
//                             RegularInstruction::UnaryMinus |
//                             RegularInstruction::UnaryPlus |
//                             RegularInstruction::BitwiseNot |
//                             RegularInstruction::Apply(_) |
//                             RegularInstruction::Is |
//                             RegularInstruction::Matches |
//                             RegularInstruction::StructuralEqual |
//                             RegularInstruction::Equal |
//                             RegularInstruction::NotStructuralEqual |
//                             RegularInstruction::NotEqual |
//                             RegularInstruction::AddAssign(_) |
//                             RegularInstruction::SubtractAssign(_) |
//                             RegularInstruction::MultiplyAssign(_) |
//                             RegularInstruction::DivideAssign(_) |
//                             RegularInstruction::CreateRef |
//                             RegularInstruction::CreateRefMut |
//                             RegularInstruction::GetRef(_) |
//                             RegularInstruction::GetLocalRef(_) |
//                             RegularInstruction::GetInternalRef(_) |
//                             RegularInstruction::GetOrCreateRef(_) |
//                             RegularInstruction::GetOrCreateRefMut(_) |
//                             RegularInstruction::AllocateSlot(_) |
//                             RegularInstruction::GetSlot(_) |
//                             RegularInstruction::DropSlot(_) |
//                             RegularInstruction::SetSlot(_) |
//                             RegularInstruction::AssignToReference(_) |
//                             RegularInstruction::Deref |
//                             RegularInstruction::TypedValue |
//                             RegularInstruction::RemoteExecution(_) |
//                             RegularInstruction::TypeExpression => unreachable!()
//                         })
//                     } else {
//                         None
//                     };
//
//                     expr.map(|expr| CollectedExecutionResult::from(expr))
//                 }
//                 Instruction::TypeInstruction(type_instruction) => {
//                     let type_instruction = collector.default_type_instruction_collection(type_instruction);
//
//                     let type_expression: Option<Type> = if let Some(type_instruction) = type_instruction {
//                         Some(match type_instruction {
//
//                             TypeInstruction::List(list_data) => {
//                                 todo!()
//                             }
//                             TypeInstruction::LiteralInteger(integer) => {
//                                 Type::structural(integer.0)
//                             }
//                             TypeInstruction::LiteralText(text_data) => {
//                                 Type::structural(text_data.0)
//                             }
//
//                             TypeInstruction::ImplType(impl_type_data) => {
//                                 let mutability: Option<ReferenceMutability> =
//                                     impl_type_data.metadata.mutability.into();
//                                 let base_type = get_next_type!(interrupt_provider);
//                                 Type::new(
//                                     TypeDefinition::ImplType(
//                                         Box::new(base_type),
//                                         impl_type_data
//                                             .impls
//                                             .iter()
//                                             .map(PointerAddress::from)
//                                             .collect(),
//                                     ),
//                                     mutability.clone(),
//                                 )
//                             }
//                             TypeInstruction::TypeReference(type_ref) => {
//                                 let metadata = type_ref.metadata;
//                                 let val = interrupt_with_maybe_value!(
//                                     interrupt_provider,
//                                     match type_ref.address {
//                                         RawPointerAddress::Local(address) => {
//                                             ExecutionInterrupt::External(
//                                                 ExternalExecutionInterrupt::ResolveLocalPointer(
//                                                     address,
//                                                 ),
//                                             )
//                                         }
//                                         RawPointerAddress::Internal(address) => {
//                                             ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolveInternalPointer(address))
//                                         }
//                                         RawPointerAddress::Full(address) => {
//                                             ExecutionInterrupt::External(
//                                                 ExternalExecutionInterrupt::ResolvePointer(
//                                                     address,
//                                                 ),
//                                             )
//                                         }
//                                     }
//                                 );
//
//                                 match val {
//                                     // simple Type value
//                                     Some(ValueContainer::Value(Value {
//                                        inner: CoreValue::Type(ty),
//                                        ..
//                                    })) => ty,
//                                     // Type Reference
//                                     Some(ValueContainer::Reference(
//                                              Reference::TypeReference(type_ref),
//                                          )) => Type::new(
//                                         TypeDefinition::Reference(type_ref),
//                                         metadata.mutability.into(),
//                                     ),
//                                     _ => {
//                                         return yield Err(ExecutionError::ExpectedTypeValue);
//                                     }
//                                 }
//                             }
//
//                             // NOTE: make sure that each possible match case is either implemented in the default collection or here
//                             // If an instruction is implemented in the default collection, it should be marked as unreachable!() here
//                             TypeInstruction::List(_) |
//                             TypeInstruction::ImplType(_) => unreachable!(),
//                         })
//                     } else {
//                         None
//                     };
//
//                     type_expression.map(|ty_expr| CollectedExecutionResult::from(ty_expr))
//                 }
//             };
//
//             if let Some(result) = result {
//                 collector.push_result(result);
//             }
//
//             // handle collecting nested expressions
//             while let Some((instruction, mut collected_results)) =
//                 collector.try_pop_collected()
//             {
//                 let expr: CollectedExecutionResult = match instruction {
//                     Instruction::RegularInstruction(
//                         regular_instruction,
//                     ) => match regular_instruction {
//                         RegularInstruction::List(_)
//                         | RegularInstruction::ShortList(_) => {
//                             let elements = yield_unwrap!(collected_results.collect_value_results_assert_existing());
//                             Some(ValueContainer::from(List::new(elements)))
//                         }
//                         RegularInstruction::Map(_)
//                         | RegularInstruction::ShortMap(_) => {
//                             let entries = yield_unwrap!(collected_results.collect_key_value_pair_results_assert_existing());
//                             Some(ValueContainer::from(Map::from(entries)))
//                         }
//                         RegularInstruction::Statements(statements_data)
//                         | RegularInstruction::ShortStatements(
//                             statements_data,
//                         ) => {
//                             let statements = collected_results.collect_value_results();
//                             match statements_data.terminated {
//                                 true => None,
//                                 false => last_value,
//                             }
//                         }
//
//                         RegularInstruction::KeyValueDynamic => {
//                             let value = collected_results.pop_value_result();
//                             let key = collected_results.pop_value_result();
//                             crate::decompiler::ast_from_bytecode::CollectedAstResult::KeyValuePair((key, value))
//                         }
//
//                         RegularInstruction::KeyValueShortText(short_text_data) => {
//                             let value = collected_results.pop_value_result();
//                             let key = DatexExpressionData::Text(short_text_data.0)
//                                 .with_default_span();
//                             crate::decompiler::ast_from_bytecode::CollectedAstResult::KeyValuePair((key, value))
//                         }
//
//                         RegularInstruction::Add
//                         | RegularInstruction::Subtract
//                         | RegularInstruction::Multiply
//                         | RegularInstruction::Divide
//                         | RegularInstruction::Matches
//                         | RegularInstruction::StructuralEqual
//                         | RegularInstruction::Equal
//                         | RegularInstruction::NotStructuralEqual
//                         | RegularInstruction::NotEqual
//                         => {
//                             let right = collected_results.pop_value_result();
//                             let left = collected_results.pop_value_result();
//                             DatexExpressionData::BinaryOperation(BinaryOperation {
//                                 operator: BinaryOperator::from(&regular_instruction),
//                                 left: Box::new(left),
//                                 right: Box::new(right),
//                                 ty: None
//                             }).with_default_span().into()
//                         }
//
//                         RegularInstruction::UnaryMinus
//                         | RegularInstruction::UnaryPlus
//                         | RegularInstruction::BitwiseNot
//                         | RegularInstruction::CreateRef
//                         | RegularInstruction::CreateRefMut
//                         | RegularInstruction::Deref
//                         => {
//                             let expr = collected_results.pop_value_result();
//                             DatexExpressionData::UnaryOperation(UnaryOperation {
//                                 operator: UnaryOperator::from(&regular_instruction),
//                                 expression: Box::new(expr),
//                             }).with_default_span().into()
//                         }
//
//                         RegularInstruction::TypedValue => {
//                             let expr = collected_results.pop_value_result();
//                             let expr_type = collected_results.pop_type_result();
//                             DatexExpressionData::ApplyChain(ApplyChain {
//                                 base: Box::new(DatexExpressionData::TypeExpression(expr_type).with_default_span()),
//                                 operations: vec![ApplyOperation::FunctionCallSingleArgument(expr)],
//                             }).with_default_span().into()
//                         }
//
//                         e => {
//                             todo!(
//                                 "Unhandled collected regular instruction: {:?}",
//                                 e
//                             );
//                         }
//                     }.into(),
//
//                     Instruction::TypeInstruction(data) => {
//                         todo!()
//                     }
//                 };
//                 collector.push_result(expr);
//             }
//
//         }
//
//     }
// }


/// Main execution loop that drives the execution of the DXB body
/// The interrupt_provider is used to provide results for synchronous or asynchronous I/O operations
pub fn execution_loop(
    state: RuntimeExecutionState,
    dxb_body: Rc<RefCell<Vec<u8>>>,
    interrupt_provider: InterruptProvider,
) -> impl Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>> {
    gen move {
        let mut instruction_iterator = iterate_instructions(dxb_body);
        let mut slots = state.slots;

        let first_instruction = instruction_iterator.next();

        let mut active_value: Option<ValueContainer> = None;

        if let Some(Ok(Instruction::RegularInstruction(first_instruction))) =
            first_instruction
        {
            // execute the root instruction, which will drive further recursive execution
            let inner_iterator = execute_regular_instruction(
                interrupt_provider.clone(),
                first_instruction,
            );
            'main: for step in inner_iterator {
                let step = yield_unwrap!(step);

                match step {
                    // yield external steps directly to be handled by the caller
                    ExecutionInterrupt::External(external_step) => {
                        yield Ok(external_step);
                    }
                    // final execution result - loop ends here
                    ExecutionInterrupt::ValueReturn(value) => {
                        return yield Ok(ExternalExecutionInterrupt::Result(
                            value,
                        ));
                    }
                    // feed new instructions to execution as long as they are requested
                    ExecutionInterrupt::GetNextRegularInstruction => {
                        loop {
                            match next_iter!(instruction_iterator, 'main) {
                                // feed next regular instruction
                                Ok(Instruction::RegularInstruction(
                                       next_instruction,
                                   )) => {
                                    interrupt_provider.provide_result(
                                        InterruptResult::NextRegularInstruction(
                                            next_instruction,
                                        ),
                                    );
                                }
                                // instruction is not a regular instruction - invalid program
                                Ok(_) => {
                                    yield Err(ExecutionError::InvalidProgram(
                                        InvalidProgramError::ExpectedRegularInstruction,
                                    ));
                                }
                                // instruction iterator ran out of instructions - must wait for more
                                Err(
                                    DXBParserError::ExpectingMoreInstructions,
                                ) => {
                                    yield Err(ExecutionError::IntermediateResultWithState(active_value.clone(), None));
                                    // assume that when continuing after this yield, more instructions will have been loaded
                                    // so we run the loop again to try to get the next instruction
                                    continue;
                                }
                                // other parsing errors from instruction iterator
                                Err(err) => {
                                    return yield Err(
                                        ExecutionError::DXBParserError(err),
                                    );
                                }
                            };
                            // only run this once per default
                            break;
                        }
                    }
                    ExecutionInterrupt::GetNextTypeInstruction => {
                        loop {
                            match next_iter!(instruction_iterator, 'main) {
                                // feed next type instruction
                                Ok(Instruction::TypeInstruction(
                                       next_instruction,
                                   )) => {
                                    interrupt_provider.provide_result(
                                        InterruptResult::NextTypeInstruction(
                                            next_instruction,
                                        ),
                                    );
                                }
                                // instruction is not a type instruction - invalid program
                                Ok(_) => {
                                    yield Err(ExecutionError::InvalidProgram(
                                        InvalidProgramError::ExpectedTypeInstruction,
                                    ));
                                }
                                // instruction iterator ran out of instructions - must wait for more
                                Err(
                                    DXBParserError::ExpectingMoreInstructions,
                                ) => {
                                    yield Err(ExecutionError::IntermediateResultWithState(active_value.clone(), None));
                                    // assume that when continuing after this yield, more instructions will have been loaded
                                    // so we run the loop again to try to get the next instruction
                                    continue;
                                }
                                // other parsing errors from instruction iterator
                                Err(err) => {
                                    return yield Err(
                                        ExecutionError::DXBParserError(err),
                                    );
                                }
                            }
                            // only run this once per default
                            break;
                        }
                    }
                    ExecutionInterrupt::GetSlotValue(address) => {
                        // if address is >= 0xffffff00, resolve internal slot
                        if address >= 0xffffff00 {
                            yield Ok(ExternalExecutionInterrupt::GetInternalSlotValue(
                                address,
                            ));
                        }
                        // else handle normal slot
                        else {
                            let val =
                                yield_unwrap!(slots.get_slot_value(address));
                            interrupt_provider.provide_result(
                                InterruptResult::ResolvedValue(val),
                            );
                        }
                    }
                    ExecutionInterrupt::SetSlotValue(address, value) => {
                        yield_unwrap!(slots.set_slot_value(address, value));
                    }
                    ExecutionInterrupt::DropSlot(address) => {
                        yield_unwrap!(slots.drop_slot(address));
                    }
                    ExecutionInterrupt::AllocateSlot(address, value) => {
                        slots.allocate_slot(address, Some(value));
                    }
                    ExecutionInterrupt::SetActiveValue(value) => {
                        active_value = value;
                    }
                    // only for internal interrupts
                    ExecutionInterrupt::TypeReturn(_) => unreachable!(),
                    ExecutionInterrupt::KeyValuePairReturn(_) => unreachable!(),
                    ExecutionInterrupt::StatementsEnd(_) => unreachable!(),
                }
            }
        } else {
            // the first instruction must always be a regular instruction
            return yield Err(ExecutionError::InvalidProgram(
                InvalidProgramError::ExpectedRegularInstruction,
            ));
        }

        // TODO: should this be unreachable?
        // if execution exited without value return, return None
        yield Ok(ExternalExecutionInterrupt::Result(None))
    }
}



// TODO
fn handle_apply(
    callee: &ValueContainer,
    args: &[ValueContainer],
) -> Result<Option<ValueContainer>, ExecutionError> {
    // callee is guaranteed to be Some here
    // apply_single if one arg, apply otherwise
    Ok(if args.len() == 1 {
        callee.apply_single(&args[0])?
    } else {
        callee.apply(args)?
    })
}
