use core::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::global::protocol_structures::instructions::{RawPointerAddress, TypeInstruction};
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::execution_loop::{ExecutionInterrupt, ExternalExecutionInterrupt, InterruptProvider};
use crate::runtime::execution::{ExecutionError, InvalidProgramError};
use crate::runtime::execution::macros::{interrupt_with_maybe_value, interrupt_with_value};
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

/// Yield an interrupt and get the next type instruction,
/// expecting the next input to be a NextTypeInstruction variant
macro_rules! interrupt_with_next_type_instruction {
    ($input:expr) => {{
        use crate::runtime::execution::execution_loop::ExecutionInterrupt;
        use crate::runtime::execution::macros::interrupt;

        let res = interrupt!($input, ExecutionInterrupt::GetNextTypeInstruction).unwrap();
        match res {
            InterruptProvider::NextTypeInstruction(value) => value,
            _ => unreachable!(), // must be ensured by execution loop
        }
    }}
}
pub(crate) use interrupt_with_next_type_instruction;

/// Drives the type instruction iteration to get the next Type value
/// Returns the resolved Type or aborts with an ExecutionError if no type could be resolved (should not happen in valid program)
macro_rules! get_next_type {
    ($interrupt_provider:expr) => {{
        use crate::runtime::execution::execution_loop::type_instruction_execution::execute_type_instruction;
        use crate::runtime::execution::execution_loop::type_instruction_execution::interrupt_with_next_type_instruction;
        use crate::runtime::execution::macros::intercept_step;

        use crate::runtime::execution::execution_loop::ExecutionInterrupt;
        use crate::runtime::execution::errors::ExecutionError;
        use crate::runtime::execution::errors::InvalidProgramError;

        let next = interrupt_with_next_type_instruction!($interrupt_provider);
        let mut inner_iterator = execute_type_instruction($interrupt_provider, next);
        let maybe_type = intercept_step!(
            inner_iterator,
            Ok(ExecutionInterrupt::TypeReturn(base_type)) => {
                base_type
            }
        );
        match maybe_type {
            Some(ty) => ty,
            None => {
                return yield Err(ExecutionError::InvalidProgram(InvalidProgramError::ExpectedTypeValue));
            }
        }
    }};
}
pub(crate) use get_next_type;

pub(crate) fn execute_type_instruction(
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
    instruction: TypeInstruction,
) -> Box<impl Iterator<Item = Result<ExecutionInterrupt, ExecutionError>>> {
    Box::new(gen move {
        yield Ok(ExecutionInterrupt::TypeReturn(match instruction {
            TypeInstruction::List(list_data) => {
                todo!()
            }
            TypeInstruction::LiteralInteger(integer) => {
                Type::structural(integer.0)
            }
            TypeInstruction::ImplType(impl_type_data) => {
                let mutability: Option<ReferenceMutability> = impl_type_data.metadata.mutability.into();
                let base_type = get_next_type!(interrupt_provider);
                Type::new(
                    TypeDefinition::ImplType(
                        Box::new(base_type),
                        impl_type_data.impls.iter().map(PointerAddress::from).collect()
                    ),
                    mutability.clone()
                )
            }
            TypeInstruction::TypeReference(type_ref) => {
                let metadata = type_ref.metadata;
                let val = interrupt_with_maybe_value!(
                        interrupt_provider,
                        match type_ref.address {
                            RawPointerAddress::Local(address) => {
                                ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolveLocalPointer(address))
                            }
                            RawPointerAddress::Internal(address) => {
                                ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolveInternalPointer(address))
                            }
                            RawPointerAddress::Full(address) => {
                                ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolvePointer(address))
                            }
                        }
                    );

                match val {
                    // simple Type value
                    Some(ValueContainer::Value(Value {inner: CoreValue::Type(ty), ..})) => {
                        ty
                    }
                    // Type Reference
                    Some(ValueContainer::Reference(Reference::TypeReference(type_ref))) => {
                        Type::new(TypeDefinition::Reference(type_ref), metadata.mutability.into())
                    }
                    _ => {
                        return yield Err(ExecutionError::ExpectedTypeValue);
                    }
                }
            }
            _ => core::todo!("#405 Undescribed by author."),
        }))
    })
}