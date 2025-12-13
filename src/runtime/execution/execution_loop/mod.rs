mod operations;
pub mod regular_instruction_iteration;
pub mod state;
pub mod type_instruction_iteration;

use core::cell::RefCell;
use crate::stdlib::rc::Rc;
use log::info;
use datex_core::runtime::execution::execution_loop::regular_instruction_iteration::next_regular_instruction_iteration;
use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator, BitwiseUnaryOperator, ComparisonOperator, LogicalUnaryOperator, ReferenceUnaryOperator, UnaryOperator};
use crate::global::operators::binary::{ArithmeticOperator, BitwiseOperator, LogicalOperator};
use crate::global::protocol_structures::instructions::{ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, RegularInstruction, IntegerData, RawFullPointerAddress, RawInternalPointerAddress, RawLocalPointerAddress, RawPointerAddress, ShortTextData, SlotAddress, TextData, TypeInstruction, Instruction};
use crate::parser::body;
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::{ExecutionError, ExecutionInput, InvalidProgramError};
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::execution_loop::type_instruction_iteration::{next_type_instruction_iteration};
use crate::runtime::execution::macros::{handle_steps, intercept_steps, interrupt, interrupt_with_maybe_value, next_iter, yield_unwrap};
use crate::traits::apply::Apply;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::list::List;
use crate::values::core_values::map::{Map, OwnedMapKey};
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

#[derive(Debug)]
pub enum ExecutionStep {
    ValueReturn(Option<ValueContainer>),
    TypeReturn(Type),
    KeyValuePairReturn((OwnedMapKey, ValueContainer)),
    AllocateSlot(u32, ValueContainer),
    GetSlotValue(u32),
    SetSlotValue(u32, ValueContainer),
    DropSlot(u32),
    GetNextRegularInstruction,
    GetNextTypeInstruction,
    External(ExternalExecutionStep)
}

// TODO ExecutionStep::External
#[derive(Debug)]
pub enum ExternalExecutionStep {
    Result(Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveLocalPointer(RawLocalPointerAddress),
    ResolveInternalPointer(RawInternalPointerAddress),
    GetInternalSlotValue(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Apply(ValueContainer, Vec<ValueContainer>),
    Pause,
}

#[derive(Debug)]
pub enum InterruptProvider {
    ResolvedValue(Option<ValueContainer>),
    NextRegularInstruction(RegularInstruction),
    NextTypeInstruction(TypeInstruction),
}

/// Main execution loop that drives the execution of the DXB body
/// The interrupt_provider is used to provide synchronous or asynchronous I/O operations
pub fn execute_loop(
    input: ExecutionInput,
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
) -> impl Iterator<Item = Result<ExternalExecutionStep, ExecutionError>> {
    gen move {
        let dxb_body = input.dxb_body;
        let state = input.state;
        let next_instructions_stack =
            state.borrow_mut().next_instructions_stack.clone();

        let mut instruction_iterator =
            body::iterate_instructions(dxb_body, next_instructions_stack);


        let first_instruction = instruction_iterator.next();

        if let Some(Ok(Instruction::RegularInstruction(first_instruction))) = first_instruction {
            let mut inner_iterator = next_regular_instruction_iteration(
                interrupt_provider.clone(),
                first_instruction,
            );
            while let Some(step) = inner_iterator.next() {
                let step = yield_unwrap!(step);
                match step {
                    // yield external steps directly
                    ExecutionStep::External(external_step) => {
                        yield Ok(external_step);
                    }
                    // final outer value return
                    ExecutionStep::ValueReturn(value) => {
                        return yield Ok(ExternalExecutionStep::Result(value))
                    }
                    // feed new instructions as long as they are requested
                    ExecutionStep::GetNextRegularInstruction => {
                        match yield_unwrap!(next_iter!(instruction_iterator)) {
                            Instruction::RegularInstruction(next_instruction) => {
                                interrupt_provider.borrow_mut().replace(
                                    InterruptProvider::NextRegularInstruction(
                                        next_instruction,
                                    ),
                                );
                            }
                            _ => unreachable!()
                        }
                    }
                    ExecutionStep::GetNextTypeInstruction => {
                        match yield_unwrap!(next_iter!(instruction_iterator)) {
                            Instruction::TypeInstruction(next_instruction) => {
                                interrupt_provider.borrow_mut().replace(
                                    InterruptProvider::NextTypeInstruction(
                                        next_instruction,
                                    ),
                                );
                            }
                            _ => unreachable!()
                        }
                    }
                    _ => todo!(),
                }
            }
        } else {
            // the first instruction must always be a regular instruction
            return yield Err(ExecutionError::InvalidProgram(
                InvalidProgramError::ExpectedRegularInstruction,
            ));
        }

        // if execution exited without value return, return None
        yield Ok(ExternalExecutionStep::Result(None))
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
