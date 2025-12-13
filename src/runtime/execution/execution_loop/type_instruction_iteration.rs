use core::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::global::protocol_structures::instructions::{RawPointerAddress, TypeInstruction};
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::execution_loop::{ExecutionStep, InterruptProvider};
use crate::runtime::execution::ExecutionError;
use crate::runtime::execution::macros::{intercept_steps, interrupt_with_next_type_instruction, interrupt_with_result, next_iter};
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

pub fn get_type_from_instructions(
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
    mut iterator: impl Iterator<Item = TypeInstruction>,
) -> impl Iterator<Item = Result<ExecutionStep, ExecutionError>> {
    gen move {
        while let Some(instruction) = iterator.next() {
            let inner_iterator = resolve_type_from_type_instruction(
                interrupt_provider.clone(),
                instruction,
            );
            intercept_steps!(
                inner_iterator,
                Ok(ExecutionStep::NextTypeInstruction) => {
                    let next_instruction = next_iter!(iterator);
                    interrupt_provider.borrow_mut().replace(
                        InterruptProvider::NextTypeInstruction(
                            next_instruction,
                        ),
                    );
                }
            )
        }
    }
}


fn resolve_type_from_type_instruction(
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
    instruction: TypeInstruction,
) -> Box<impl Iterator<Item = Result<ExecutionStep, ExecutionError>>> {
    Box::new(gen move {
        match instruction {
            TypeInstruction::ListStart => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::Pause
                );
            }
            TypeInstruction::LiteralInteger(integer) => {
                yield Ok(ExecutionStep::InternalTypeReturn(
                    Type::structural(integer.0),
                ));
            }
            TypeInstruction::ImplType(impl_type_data) => {
                let mutability: Option<ReferenceMutability> = impl_type_data.metadata.mutability.into();
                let next = interrupt_with_next_type_instruction!(
                    interrupt_provider,
                    ExecutionStep::NextTypeInstruction
                );
                let inner_iterator = resolve_type_from_type_instruction(interrupt_provider, next);
                intercept_steps!(
                    inner_iterator,
                    Ok(ExecutionStep::InternalTypeReturn(base_type)) => {
                        yield Ok(ExecutionStep::InternalTypeReturn(
                            Type::new(TypeDefinition::ImplType(Box::new(base_type), impl_type_data.impls.iter().map(PointerAddress::from).collect()), mutability.clone()))
                        );
                    }
                );
            }
            TypeInstruction::TypeReference(type_ref) => {
                let metadata = type_ref.metadata;
                let val = interrupt_with_result!(
                        interrupt_provider,
                        match type_ref.address {
                            RawPointerAddress::Local(address) => {
                                ExecutionStep::ResolveLocalPointer(address)
                            }
                            RawPointerAddress::Internal(address) => {
                                ExecutionStep::ResolveInternalPointer(address)
                            }
                            RawPointerAddress::Full(address) => {
                                ExecutionStep::ResolvePointer(address)
                            }
                        }
                    );

                match val {
                    // simple Type value
                    Some(ValueContainer::Value(Value {inner: CoreValue::Type(ty), ..})) => {
                        yield Ok(ExecutionStep::InternalTypeReturn(ty));
                    }
                    // Type Reference
                    Some(ValueContainer::Reference(Reference::TypeReference(type_ref))) => {
                        yield Ok(ExecutionStep::InternalTypeReturn(Type::new(TypeDefinition::Reference(type_ref), metadata.mutability.into())));
                    }
                    _ => {
                        return yield Err(ExecutionError::ExpectedTypeValue);
                    }
                }

            }
            _ => core::todo!("#405 Undescribed by author."),
        }
    })
}