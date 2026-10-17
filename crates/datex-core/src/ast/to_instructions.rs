use crate::{
    ast::expressions::{
        Apply, BinaryOperation, ComparisonOperation, CreateShared,
        DatexExpressionData, DeriveRef, DeriveSharedRef, GenericInstantiation,
        InterfaceMethodCall, List, Map, PropertyAccess, PropertyAssignment,
        RangeDeclaration, RequestSharedRef, RootPropertyAccess, TagExpression,
        UnaryOperation, Unbox, UnboxAssignment,
    },
    core_compiler::{
        to_instructions::ToInstructions,
    },
    global::{operators::ModificationOperator, root_properties::RootProperty},
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    shared_values::{ReferenceMutability, SharedContainerMutability},
};
use core::str::FromStr;
use log::info;
use crate::ast::expressions::{CallableDeclaration, Conditional, DatexExpression, RemoteExecution, Statements, UnboundedStatement, ValueAccessType, VariableAccess, VariableAssignment, VariableDeclaration, VariableKind};
use crate::compiler::{compile_child_realm_instructions, Variable, VariableModel};
use crate::compiler::ast_node_to_instructions::AstNodeToInstructions;
use crate::compiler::context::{CompilationContext};
use crate::compiler::error::CompilerError;
use crate::compiler::metadata::CompileContextMetadata;
use crate::compiler::precompiler::precompiled_ast::{AstMetadata, RichAst};
use crate::core_compiler::core_compilation_context::DXBWithSharedValues;
use crate::core_compiler::to_instructions::{yield_all, yield_all_ok, yield_all_ok_collected};
use crate::global::protocol_structures::injected_values::{InjectedValueType, LocalInjectedValueType, SharedInjectedValueType};
use crate::global::stack_index::StackIndex;
use crate::instruction::instruction_codes::InstructionCode;
use crate::instruction::instruction_data::{CallableDeclarationData, CallableSignatureData, JumpData, ShortTextData, UnboundedStatementsData};
use crate::shared_values::SharedContainer;
use crate::shared_values::traits::SharedContainerCommon;
use crate::values::value_container::ValueContainer;

macro yield_unwrap($expr:expr) {
    match $expr {
        Ok(val) => val,
        Err(e) => return yield Err(e),
    }
}

impl AstNodeToInstructions for DatexExpression {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a> {
        Box::new(gen move {
            yield_all!(self.data.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}
impl AstNodeToInstructions for DatexExpressionData {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            match self {
                DatexExpressionData::Integer(integer) => {
                    yield Ok(RegularInstruction::Integer(integer.clone()).into())
                }
                DatexExpressionData::TypedInteger(typed_integer) => {
                    yield_all_ok!(typed_integer.to_instructions(compilation_context));
                }

                DatexExpressionData::Instant(instant) => {
                    yield_all_ok!(instant.to_instructions(compilation_context));
                }

                DatexExpressionData::Decimal(decimal) => {
                    yield_all_ok!(decimal.to_instructions(compilation_context));
                }

                DatexExpressionData::TypedDecimal(typed_decimal) => {
                    yield_all_ok!(typed_decimal.to_instructions(compilation_context));
                }

                DatexExpressionData::Text(text) => {
                    yield_all_ok!(text.to_instructions(compilation_context));
                }

                DatexExpressionData::Boolean(boolean) => {
                    yield_all_ok!(boolean.to_instructions(compilation_context));
                }

                DatexExpressionData::Endpoint(endpoint) => {
                    yield_all_ok!(endpoint.to_instructions(compilation_context));
                }

                DatexExpressionData::Null => {
                    yield Ok(RegularInstruction::null().into());
                }

                DatexExpressionData::List(list) => {
                    yield_all!(list.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::Map(map) => {
                    yield_all!(map.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // unary operations
                DatexExpressionData::UnaryOperation(unary_operation) => {
                    yield_all!(unary_operation.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // binary operations
                DatexExpressionData::BinaryOperation(binary_operation) => {
                    yield_all!(binary_operation.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // comparisons
                DatexExpressionData::ComparisonOperation(
                    comparison_operation,
                ) => {
                    yield_all!(comparison_operation.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // apply
                DatexExpressionData::Apply(apply) => {
                    yield_all!(apply.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // interface methods
                DatexExpressionData::InterfaceMethodCall(
                    interface_method_call,
                ) => {
                    yield_all!(interface_method_call.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // property access
                DatexExpressionData::PropertyAccess(property_access) => {
                    yield_all!(property_access.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::GenericInstantiation(
                    generic_instantiation,
                ) => {
                    yield_all!(generic_instantiation.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::PropertyAssignment(
                    property_assignment,
                ) => {
                    yield_all!(property_assignment.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::RequestSharedRef(request_shared_ref) => {
                    yield_all!(request_shared_ref.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::UnboxAssignment(unbox_assignment) => {
                    yield_all!(unbox_assignment.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // root property
                DatexExpressionData::RootPropertyAccess(
                    root_property_access,
                ) => {
                    yield_all!(root_property_access.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // refs
                DatexExpressionData::DeriveRef(derive_ref) => {
                    yield_all!(derive_ref.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // shared refs
                DatexExpressionData::DeriveSharedRef(derive_shared_ref) => {
                    yield_all!(derive_shared_ref.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // shared values
                DatexExpressionData::CreateShared(create_shared) => {
                    yield_all!(create_shared.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::TypeExpression(type_expression) => {
                    yield Ok(RegularInstruction::TypeExpression.into());
                    yield_all!(type_expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::Range(range_dec) => {
                    yield_all!(range_dec.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::Unbox(unbox) => {
                    yield_all!(unbox.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::Tag(tag_expression) => {
                    yield_all!(tag_expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::ResolveCoreLibId(core_lib_id) => {
                    yield Ok(RegularInstruction::get_core_lib_value(
                        (*core_lib_id).into(),
                    )
                    .into());
                }

                DatexExpressionData::CallableDeclaration(
                    callable_declaration,
                ) => {
                    yield_all!(callable_declaration.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::Conditional(conditional) => {
                    yield_all!(conditional.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }
                DatexExpressionData::RemoteExecution(remote_execution) => {
                    yield_all!(remote_execution.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::VariableDeclaration(
                    _variable_declaration,
                ) => {
                    yield_all!(_variable_declaration.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }
                DatexExpressionData::Statements(statements) => {
                    yield_all!(statements.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::VariableAssignment(
                    variable_assignment,
                ) => {
                    yield_all!(variable_assignment.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::VariableAccess(variable_access) => {
                    yield_all!(variable_access.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                DatexExpressionData::Placeholder(value_access_type) => {
                    yield_all!(value_access_type.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                e => panic!(
                    "Expression to instruction not implemented for {:?}",
                    e
                ),
            }
        })
    }
}

/// implementation used for [DatexExpressionData::Placeholder]
impl AstNodeToInstructions for ValueAccessType {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a> {
        Box::new(gen move {
            // FIXME #720
            let placeholder = compilation_context
                .inserted_values
                .get_mut(compilation_context.inserted_value_index)
                .expect("Placeholder index out of bounds");
            if let Some(value_container) = placeholder.take() {
                // TODO: validate in precompiler that the value container is actually a shared value

                match value_container {
                    ValueContainer::Local(value) => match self {
                        ValueAccessType::SharedRef
                        | ValueAccessType::SharedRefMut => {
                            return yield Err(
                                CompilerError::SharedRefToNonSharedValue,
                            );
                        }
                        ValueAccessType::MoveOrCopy => {
                            yield_all_ok_collected!(value.to_instructions(compilation_context));
                        }
                        ValueAccessType::Clone => {
                            yield_all_ok_collected!(value.to_instructions(compilation_context));
                        }
                        ValueAccessType::Borrow => {
                            yield_all_ok_collected!(value.to_instructions(compilation_context));
                        }
                    },
                    ValueContainer::Shared(shared_container) => {
                        match self {
                            ValueAccessType::SharedRefMut => {
                                let shared_container_mut_ref = yield_unwrap!(
                                    shared_container
                                    .try_derive_mutable_reference()
                                    .map_err(|_| CompilerError::SharedMutRefToImmutableValue)
                                );
                                yield_all_ok_collected!(shared_container_mut_ref.to_instructions(compilation_context));
                            }
                            ValueAccessType::SharedRef => {
                                let shared_container_immut_ref = shared_container.derive_immutable_reference();
                                yield_all_ok_collected!(shared_container_immut_ref.to_instructions(compilation_context));
                            },
                            ValueAccessType::MoveOrCopy => {
                                match shared_container {
                                    SharedContainer::Owned(shared_container) => {
                                        yield_all_ok_collected!(shared_container.to_instructions(compilation_context));
                                    }
                                    _ => return yield Err(CompilerError::InvalidConversionFromRefToOwnedValue),
                                }
                            },
                            ValueAccessType::Clone => {
                                let cloned = shared_container.inner().base_shared_container().value_container().clone();
                                match cloned {
                                    ValueContainer::Local(value) => {
                                        yield_all_ok_collected!(value.to_instructions(compilation_context));
                                    }
                                    ValueContainer::Shared(shared_container) => {
                                        yield_all_ok_collected!(shared_container.to_instructions(compilation_context));
                                    }
                                }
                            },
                            ValueAccessType::Borrow => {
                                let reference = shared_container.derive_reference_with_max_mutability();
                                yield_all_ok_collected!(reference.to_instructions(compilation_context));
                            }
                        };
                    }
                }
            } else {
                // TODO
                // compilation_context
                //     .append_instruction_code(InstructionCode::CLONE_STACK_VALUE);
                // compilation_context.insert_virtual_slot_address(
                //     InjectedParentVariable::local(
                //         compilation_context.inserted_value_index as u32,
                //     ),
                // );
            }
            compilation_context.inserted_value_index += 1;
        })
    }
}

impl AstNodeToInstructions for Conditional {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let Conditional {
                condition,
                then_branch,
                else_branch,
            } = self;

            compilation_context.mark_has_non_static_value();
            let input = compilation_context.core_context.input.clone();


            let condition_instructions = {
                // FIXME: persist shared value tracking across contexts
                let mut inner_ctx = CompilationContext::new(
                    compilation_context.inserted_values.clone(),
                    compilation_context.execution_mode,
                    input.clone(),
                    compilation_context.scope.clone(),
                );

                condition.ast_node_to_instructions(
                    &mut inner_ctx,
                    ast_metadata,
                    CompileContextMetadata::default(),
                ).collect::<Vec<_>>()
            };

            let then_instructions = {
                let mut inner_ctx = CompilationContext::new(
                    compilation_context.inserted_values.clone(),
                    compilation_context.execution_mode,
                    input.clone(),
                    compilation_context.scope.clone(),
                );

                then_branch.ast_node_to_instructions(
                    &mut inner_ctx,
                    ast_metadata,
                    CompileContextMetadata::default(),
                ).collect::<Vec<_>>()
            };

            let else_instructions = match else_branch {
                Some(else_expr) => {
                    let mut inner_ctx = CompilationContext::new(
                        compilation_context.inserted_values.clone(),
                        compilation_context.execution_mode,
                        input,
                        compilation_context.scope.clone()
                    );

                    else_expr.ast_node_to_instructions(
                        &mut inner_ctx,
                        ast_metadata,
                        CompileContextMetadata::default(),
                    ).collect::<Vec<_>>()
                }
                None => Vec::new(),
            };
            yield Ok(RegularInstruction::UnboundedStatements.into());

            yield Ok(RegularInstruction::JumpIfFalse(
                JumpData {
                    // FIXME: offset in instructions vs bytes
                    // The false path starts after the then branch and, only
                    // when present, the five-byte Jump used by the true path
                    // to skip the else branch.
                    offset: (then_instructions.len()
                        + if else_instructions.is_empty() { 0 } else { 5 })
                        as i32,
                },
            ).into());

            yield_all!(condition_instructions);
            yield_all!(then_instructions);

            if !else_instructions.is_empty() {
                yield Ok(RegularInstruction::Jump(JumpData {
                    offset: else_instructions.len() as i32,
                }).into());
                yield_all!(else_instructions);
            }

            yield Ok(
                RegularInstruction::UnboundedStatementsEnd(
                    UnboundedStatementsData { terminated: false },
                ).into()
            );
        })
    }
}

impl AstNodeToInstructions for RemoteExecution {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let RemoteExecution {
                left: caller,
                right: ast,
                injected_variable_count,
            } = self;

            compilation_context.mark_has_non_static_value();

            let (instruction_block_data, new_scope) = yield_unwrap!(
                compile_child_realm_instructions(
                    compilation_context,
                    ast,
                    injected_variable_count.unwrap(), // must be set by precompiler
                    ast_metadata,
                    vec![],
                )
            );

            compilation_context.scope = new_scope;

            // insert remote execution instruction
            yield Ok(RegularInstruction::remote_execution(
                instruction_block_data,
            ).into());

            // insert compiled caller expression
            yield_all!(
                caller.ast_node_to_instructions_inner(compilation_context, ast_metadata)
            );
        })
    }
}

impl AstNodeToInstructions for VariableAccess {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let VariableAccess {
                name,
                access_type,
                ..
            } = self;

            compilation_context.mark_has_non_static_value();

            let slot_type = match access_type {
                ValueAccessType::SharedRefMut => {
                    InjectedValueType::Shared(SharedInjectedValueType::RefMut)
                }
                ValueAccessType::SharedRef => {
                    InjectedValueType::Shared(SharedInjectedValueType::Ref)
                }
                // TODO: map to local slot types depending on type
                ValueAccessType::MoveOrCopy => {
                    InjectedValueType::Shared(SharedInjectedValueType::Move)
                }
                // TODO:
                ValueAccessType::Clone => {
                    InjectedValueType::Local(LocalInjectedValueType::Move)
                }
                ValueAccessType::Borrow => {
                    InjectedValueType::Shared(SharedInjectedValueType::Move)
                }
            };

            // get variable slot address
            let (stack_index, ..) = yield_unwrap!(
                compilation_context
                    .scope
                    .resolve_variable_name_with_slot_type(&name, slot_type)
                    .ok_or_else(|| {
                        CompilerError::UndeclaredVariable(name.clone())
                    })
            );

            // append binary code to load variable
            let slot_access_instruction = match access_type {
                ValueAccessType::SharedRefMut => {
                    RegularInstruction::get_stack_value_shared_ref_mut(stack_index)
                }
                ValueAccessType::SharedRef => {
                    RegularInstruction::get_stack_value_shared_ref(stack_index)
                }
                ValueAccessType::MoveOrCopy => {
                    RegularInstruction::take_stack_value(stack_index)
                }
                ValueAccessType::Clone => RegularInstruction::clone_stack_value(stack_index),
                ValueAccessType::Borrow => RegularInstruction::borrow_stack_value(stack_index),
            };

            yield Ok(slot_access_instruction.into());
        })
    }
}

impl AstNodeToInstructions for VariableAssignment {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let VariableAssignment {
                operator,
                name,
                expression,
                ..
            } = self;

            compilation_context.mark_has_non_static_value();
            // get variable slot address
            let (stack_index, kind) = yield_unwrap!(yield_unwrap!(
                    compilation_context
                    .scope
                    .resolve_variable_name(&name, None)
                    .map_err(|_| {
                        CompilerError::AssignmentToExternalVariable(name.clone())
                    })
                )
                .ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                }));

            // TODO #484: check not needed, is already handled in precompiler - can we guarantee this?
            // if const, return error
            if kind == VariableKind::Const {
                return yield Err(CompilerError::AssignmentToConst(name.clone()));
            }

            match operator {
                None => {
                    // append binary code to load variable
                    info!(
                        "append variable - stack index: {stack_index:?}, name: {name}"
                    );
                    yield Ok(RegularInstruction::set_stack_value(stack_index).into());

                    // compile expression
                    yield_all!(expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }
                Some(operator) => {
                    // TODO #435: handle mut type
                    // // if immutable reference, return error
                    // if mut_type == Some(ReferenceMutability::Immutable) {
                    //     return Err(
                    //         CompilerError::AssignmentToImmutableReference(
                    //             name.clone(),
                    //         ),
                    //     );
                    // }
                    // // if immutable value, return error
                    // else if mut_type == None {
                    //     return Err(CompilerError::AssignmentToImmutableValue(
                    //         name.clone(),
                    //     ));
                    // }
                    
                    let direct_assignment_operator = match operator {
                        ModificationOperator::AddAssign => Some(RegularInstruction::increment()),
                        ModificationOperator::SubtractAssign => Some(RegularInstruction::decrement()),
                        _operator => None,
                    };

                    if let Some(instruction) = direct_assignment_operator {
                        yield Ok(instruction.into());
                        
                        // compile expression
                        yield_all!(expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));

                        yield Ok(RegularInstruction::borrow_stack_value(stack_index).into());
                    } else {
                        // Generate x = x * z instructions;
                        todo!()
                    }
                }
            }
        })
    }
}


impl AstNodeToInstructions for VariableDeclaration {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let VariableDeclaration {
                id,
                name,
                kind,
                type_annotation: _,
                init_expression: value,
            } = self;

            compilation_context.mark_has_non_static_value();

            // push to stack
            yield Ok(RegularInstruction::push_to_stack().into());
            
            // compile expression
            yield_all!(value.ast_node_to_instructions_inner(compilation_context, ast_metadata));

            let stack_index = compilation_context.scope.get_next_stack_index();

            let variable_model =
                VariableModel::infer_from_ast_metadata_and_type(
                    ast_metadata,
                    id,
                    kind,
                    compilation_context.execution_mode,
                );

            // create new variable depending on the model
            let variable = match variable_model {
                VariableModel::Constant => {
                    Variable::new_const(name.clone(), stack_index)
                }
                VariableModel::VariableSlot => {
                    Variable::new_variable_slot(name.clone(), *kind, stack_index)
                }
            };

            compilation_context.scope.register_variable_slot(variable);
        })
    }
}

impl AstNodeToInstructions for Statements {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let Statements {
                statements,
                is_terminated,
                unbounded,
            } = self;
            
            compilation_context.mark_has_non_static_value();
            // if single statement and not terminated, just compile the expression
            // (not for unbounded execution mode)
            if unbounded.is_none() && statements.len() == 1 && !is_terminated {
                yield_all!(statements[0].ast_node_to_instructions_inner(compilation_context, ast_metadata));
            } else {
                let is_outer_context = ctx_metadata.is_outer_context();

                // Enter a child scope for nested contexts.
                if !is_outer_context {
                    let parent_scope =
                        core::mem::take(&mut compilation_context.scope);
                    compilation_context.scope = parent_scope.push();
                }

                if let Some(UnboundedStatement { is_first, .. }) = unbounded {
                    // if this is the first section of an unbounded statements block, mark as unbounded
                    if *is_first {
                        yield Ok(RegularInstruction::unbounded_statements().into());
                    }
                    // if not first, don't insert any instruction code
                }
                // otherwise, statements with fixed length
                else {
                    yield Ok(RegularInstruction::statements(
                        statements.len() as u32,
                        *is_terminated,
                    ).into());
                }

                for statement in statements.into_iter() {
                    yield_all!(statement.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }

                // Restore the parent scope after compiling the child context.
                if !is_outer_context {
                    let child_scope =
                        core::mem::take(&mut compilation_context.scope);

                    let last_scope = child_scope
                        .pop()
                        .ok_or(CompilerError::ScopePopError);

                    compilation_context.scope = yield_unwrap!(last_scope);
                }

                // if this is the last section of an unbounded statements block, add closing instruction
                if let Some(UnboundedStatement { is_last: true, .. }) =
                    unbounded
                {
                    yield Ok(RegularInstruction::unbounded_statements_end(*is_terminated).into());
                }
            }
        })
    }
}

impl AstNodeToInstructions for CallableDeclaration {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let CallableDeclaration {
                signature,
                body,
                injected_variable_count,
            } = self;

            compilation_context.mark_has_non_static_value();

            // generate variables for parameters
            let mut variables = vec![];
            let mut index = StackIndex(0);
            for (name, _) in &signature.parameters {
                variables.push(
                    Variable::new_const(name.clone(), index), // TODO: allow var
                );
                index += 1;
            }
            if let Some((name, _)) = &signature.rest_parameter {
                variables.push(
                    Variable::new_const(name.clone(), index), // TODO: allow var
                );
            }
            
            let res = compile_child_realm_instructions(
                compilation_context,
                body,
                injected_variable_count.unwrap(), // must be set by precompiler
                ast_metadata,
                variables,
            );

            let (instruction_block_data, new_scope) = yield_unwrap!(res);

            compilation_context.scope = new_scope;

            yield Ok(RegularInstruction::CallableDeclaration(
                CallableDeclarationData {
                    signature: CallableSignatureData {
                        name: ShortTextData(signature.name.clone().unwrap_or_default()),
                        kind: signature.kind,
                        requires_async: signature.requires_async,
                        parameter_count: signature.parameters.len() as u8,
                        has_rest_parameter: signature.rest_parameter.is_some(),
                        has_return_type: signature.return_type.is_some(),
                        has_yeet_type: signature.yeet_type.is_some(),
                        parameter_names: signature
                            .parameters
                            .iter()
                            .map(|(name, _)| ShortTextData(name.clone()))
                            .collect(),
                        rest_parameter_name: signature
                            .rest_parameter
                            .clone()
                            .map(|(name, _)| ShortTextData(name)),
                    },
                    body: instruction_block_data,
                },
            ).into());

            // add parameter types
            for (_, param) in &signature.parameters {
                yield_all!(param.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
            // add rest parameter type
            if let Some((_, param)) = &signature.rest_parameter {
                yield_all!(param.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
            // add return type
            if let Some(ty) = &signature.return_type {
                yield_all!(ty.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
            // add yield type
            if let Some(ty) = &signature.yeet_type {
                yield_all!(ty.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
        })
    }
}


impl AstNodeToInstructions for RangeDeclaration {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            yield Ok(RegularInstruction::range().into());
            yield_all!(self.start.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            yield_all!(self.end.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for ComparisonOperation {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();

            yield Ok(RegularInstruction::comparison_operation(self.operator)
                .into());
            yield_all!(self.left.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            yield_all!(self.right.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for UnboxAssignment {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();

            match self.operator {
                Some(operator) => match operator {
                    ModificationOperator::AddAssign => {
                        yield Ok(RegularInstruction::increment().into())
                    }
                    ModificationOperator::SubtractAssign => {
                        yield Ok(RegularInstruction::decrement().into())
                    }
                    _ => todo!("Generate x = x * z instructions;"),
                },
                None => {
                    yield Ok(RegularInstruction::set_shared_container_value()
                        .into())
                }
            };

            // compile assigned expression
            yield_all!(self.assigned_expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));

            // compile unbox expression
            yield_all!(self.unbox_expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for PropertyAssignment {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();

            let PropertyAssignment {
                base,
                property,
                assigned_expression,
                ..
            } = self;
            // depending on the key, handle different property assignments

            match &property.data() {
                DatexExpressionData::Text(key)
                    if key.len() <= u8::MAX as usize =>
                {
                    yield Ok(RegularInstruction::set_entry_text(key.as_string())
                        .into());
                }

                DatexExpressionData::Integer(index)
                    if let Some(index) = index.as_u32() =>
                {
                    yield Ok(RegularInstruction::set_entry_index(index).into());
                }

                _ => {
                    yield Ok(RegularInstruction::set_entry_dynamic().into());
                    yield_all!(property.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }
            }
            // compile assigned expression
            yield_all!(assigned_expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));

            // compile base expression
            yield_all!(base.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for UnaryOperation {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();

            yield Ok(RegularInstruction::unary_operation(self.operator).into());
            yield_all!(self.expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for Apply {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();
            
            // more than u8 size args -> error
            let len = self.arguments.len();
            if len > u8::MAX as usize {
                return yield Err(CompilerError::TooManyApplyArguments);
            }

            yield Ok(RegularInstruction::apply(len as u8).into());
            // compile arguments
            for argument in &self.arguments {
                yield_all!(argument.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
            // compile function expression
            yield_all!(self.base.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for InterfaceMethodCall {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();
                
            // TODO: replace with trait impls
            match self.method_name.as_str() {
                "append" => {
                    // must be exactly one element
                    if self.arguments.len() != 1 {
                        return yield Err(CompilerError::InvalidInterfaceMethodCall(
                            self.method_name.to_string(),
                        ));
                    }

                    
                    yield Ok(RegularInstruction::append_entry().into());

                    for argument in &self.arguments {
                        yield_all!(argument.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                    }
                }

                "clear" => {
                    // no arguments allowed
                    if !self.arguments.is_empty() {
                        return yield Err(CompilerError::InvalidInterfaceMethodCall(
                            self.method_name.to_string(),
                        ));
                    }
                    
                    yield Ok(RegularInstruction::clear().into());
                }

                "splice" => {
                    // must be exactly three elements
                    if self.arguments.len() != 3 {
                        return yield Err(CompilerError::InvalidInterfaceMethodCall(
                            self.method_name.to_string(),
                        ));
                    }
                    
                    yield Ok(RegularInstruction::splice_dynamic().into());

                    for argument in &self.arguments {
                        yield_all!(argument.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                    }
                }

                _ => {
                    // TODO: check if method actually exists

                    yield Ok(RegularInstruction::call_method(
                        self.method_name.clone(),
                        self.arguments.len() as u8,
                    )
                    .into());

                    for argument in &self.arguments {
                        yield_all!(argument.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                    }
                }
            }

            // compile target expression
            yield_all!(self.target.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for PropertyAccess {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();

            // depending on the key, handle different property accesses
            match self.property.data() {
                // simple text key if length fits in u8
                DatexExpressionData::Text(key) if key.len() <= 255 => {
                    yield Ok(RegularInstruction::get_entry_text(key.as_string())
                        .into());
                }
                // index access if integer fits in u32
                DatexExpressionData::Integer(index)
                    if let Some(index) = index.as_u32() =>
                {
                    yield Ok(RegularInstruction::get_entry_index(index).into());
                }
                _ => {
                    yield Ok(RegularInstruction::get_entry_dynamic().into());
                    yield_all!(self.property.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }
            }

            // compile base expression
            yield_all!(self.base.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for GenericInstantiation {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            // NOTE: might already be handled in type compilation
            todo!()
        })
    }
}

impl AstNodeToInstructions for RequestSharedRef {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        compilation_context.mark_has_non_static_value();
        
        Box::new(core::iter::once(
            Ok(RegularInstruction::get_shared_ref(
                self.address.clone(),
                &self.mutability,
            )
            .into()),
        ))
    }
}

impl AstNodeToInstructions for List {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            yield Ok(RegularInstruction::list(self.items.len() as u32).into());
            for item in &self.items {
                yield_all!(item.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
        })
    }
}

impl AstNodeToInstructions for Map {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            yield Ok(RegularInstruction::map(self.entries.len() as u32).into());
            for (key, value) in &self.entries {
                match &*key.data {
                    // text -> insert key string
                    DatexExpressionData::Text(text) => {
                        if text.len() < 256 {
                            yield Ok(RegularInstruction::key_value_short_text(
                                text.as_string(),
                            )
                            .into());
                        } else {
                            yield Ok(RegularInstruction::key_value_dynamic()
                                .into());
                            yield Ok(RegularInstruction::text(text.as_string())
                                .into());
                        }
                    }
                    // other -> insert key as dynamic
                    _ => {
                        yield Ok(RegularInstruction::key_value_dynamic().into());
                        yield_all!(key.ast_node_to_instructions_inner(compilation_context, ast_metadata));
                    }
                };

                // value
                yield_all!(value.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
        })
    }
}

impl AstNodeToInstructions for TagExpression {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            yield Ok(RegularInstruction::tagged_value(
                self.tag.clone(),
                self.expression.is_none(),
            )
            .into());
            if let Some(expression) = &self.expression {
                yield_all!(expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            }
        })
    }
}
impl AstNodeToInstructions for RootPropertyAccess {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            let root_property = RootProperty::from_str(&self.property_name);

            if let Ok(root_property) = root_property {
                yield Ok(RegularInstruction::get_root_property(root_property).into());

            } else {
                return yield Err(CompilerError::InvalidRootPropertyName(
                    self.property_name.clone(),
                ));
            }
        })
    }
}

impl AstNodeToInstructions for Unbox {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            yield Ok(RegularInstruction::unbox().into());
            yield_all!(self.expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for DeriveRef {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        // TODO #764: handle lifetimes, mutability, correctly (in precompiler)
        // TODO #765: handle move/clone
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();

            yield_all!(self.expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for CreateShared {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();

            match self.mutability {
                SharedContainerMutability::Immutable => {
                    yield Ok(RegularInstruction::create_shared().into());
                }
                SharedContainerMutability::Mutable => {
                    yield Ok(RegularInstruction::create_shared_mut().into());
                }
            }

            yield_all!(self.expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for DeriveSharedRef {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            match self.mutability {
                ReferenceMutability::Immutable => {
                    yield Ok(RegularInstruction::derive_shared_reference().into());
                }
                ReferenceMutability::Mutable => {
                    yield Ok(RegularInstruction::derive_shared_reference_mut()
                        .into());
                }
            }

            yield_all!(self.expression.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}

impl AstNodeToInstructions for BinaryOperation {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>
    {
        Box::new(gen move {
            compilation_context.mark_has_non_static_value();
            yield Ok(RegularInstruction::binary_operation(self.operator).into());
            yield_all!(self.left.ast_node_to_instructions_inner(compilation_context, ast_metadata));
            yield_all!(self.right.ast_node_to_instructions_inner(compilation_context, ast_metadata));
        })
    }
}
