use crate::{
    ast::expressions::{
        Apply, BinaryOperation, ComparisonOperation, CreateShared,
        DatexExpressionData, DeriveRef, DeriveSharedRef, GenericInstantiation,
        InterfaceMethodCall, List, Map, PropertyAccess, PropertyAssignment,
        RangeDeclaration, RequestSharedRef, RootPropertyAccess, TagExpression,
        UnaryOperation, Unbox, UnboxAssignment,
    },
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    global::{operators::ModificationOperator, root_properties::RootProperty},
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    shared_values::{ReferenceMutability, SharedContainerMutability},
};
use core::str::FromStr;
use crate::ast::expressions::CallableDeclaration;
use crate::compiler::{Variable};
use crate::compiler::ast_node_to_instructions::AstNodeToInstructions;
use crate::compiler::context::CompilationContext;
use crate::compiler::error::CompilerError;
use crate::compiler::precompiler::precompiled_ast::{AstMetadata, RichAst};
use crate::compiler::scope::CompilationScope;
use crate::core_compiler::to_instructions::{yield_all, yield_all_ok};
use crate::global::stack_index::StackIndex;
use crate::instruction::instruction_data::{CallableDeclarationData, CallableSignatureData, ShortTextData};


impl AstNodeToInstructions for DatexExpressionData {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &mut CompilationContext,
        metadata: &mut AstMetadata,
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
                    yield_all_ok!(list.to_instructions(compilation_context));
                }

                DatexExpressionData::Map(map) => {
                    yield_all_ok!(map.to_instructions(compilation_context));
                }

                // unary operations
                DatexExpressionData::UnaryOperation(unary_operation) => {
                    yield_all_ok!(unary_operation.to_instructions(compilation_context));
                }

                // binary operations
                DatexExpressionData::BinaryOperation(binary_operation) => {
                    yield_all_ok!(binary_operation.to_instructions(compilation_context));
                }

                // comparisons
                DatexExpressionData::ComparisonOperation(
                    comparison_operation,
                ) => {
                    yield_all_ok!(comparison_operation.to_instructions(compilation_context));
                }

                // apply
                DatexExpressionData::Apply(apply) => {
                    yield_all_ok!(apply.to_instructions(compilation_context));
                }

                // interface methods
                DatexExpressionData::InterfaceMethodCall(
                    interface_method_call,
                ) => {
                    yield_all_ok!(interface_method_call.to_instructions(compilation_context));
                }

                // property access
                DatexExpressionData::PropertyAccess(property_access) => {
                    yield_all_ok!(property_access.to_instructions(compilation_context));
                }

                DatexExpressionData::GenericInstantiation(
                    generic_instantiation,
                ) => {
                    yield_all_ok!(generic_instantiation.to_instructions(compilation_context));
                }

                DatexExpressionData::PropertyAssignment(
                    property_assignment,
                ) => {
                    yield_all_ok!(property_assignment.to_instructions(compilation_context));
                }

                DatexExpressionData::RequestSharedRef(request_shared_ref) => {
                    yield_all_ok!(request_shared_ref.to_instructions(compilation_context));
                }

                DatexExpressionData::UnboxAssignment(unbox_assignment) => {
                    yield_all_ok!(unbox_assignment.to_instructions(compilation_context));
                }

                // root property
                DatexExpressionData::RootPropertyAccess(
                    root_property_access,
                ) => {
                    yield_all_ok!(root_property_access.to_instructions(compilation_context));
                }

                // refs
                DatexExpressionData::DeriveRef(derive_ref) => {
                    yield_all_ok!(derive_ref.to_instructions(compilation_context));
                }

                // shared refs
                DatexExpressionData::DeriveSharedRef(derive_shared_ref) => {
                    yield_all_ok!(derive_shared_ref.to_instructions(compilation_context));
                }

                // shared values
                DatexExpressionData::CreateShared(create_shared) => {
                    yield_all_ok!(create_shared.to_instructions(compilation_context));
                }

                DatexExpressionData::TypeExpression(type_expression) => {
                    yield Ok(RegularInstruction::TypeExpression.into());
                    yield_all_ok!(type_expression.to_instructions(compilation_context));
                }

                DatexExpressionData::Range(range_dec) => {
                    yield_all_ok!(range_dec.to_instructions(compilation_context));
                }

                DatexExpressionData::Unbox(unbox) => {
                    yield_all_ok!(unbox.to_instructions(compilation_context));
                }

                DatexExpressionData::Tag(tag_expression) => {
                    yield_all_ok!(tag_expression.to_instructions(compilation_context));
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
                    yield_all!(callable_declaration.ast_node_to_instructions(compilation_context, metadata));
                }

                DatexExpressionData::Conditional(_conditional) => {
                    todo!(
                        "Need actual byte positions and not instruction counts for jumps"
                    );
                }
                DatexExpressionData::RemoteExecution(_remote_execution) => {
                    todo!("Need context and scope data and stuff");
                }

                DatexExpressionData::VariableDeclaration(
                    _variable_declaration,
                ) => {
                    todo!("Need context and scope data and stuff");
                }
                DatexExpressionData::Statements(_statements) => {
                    todo!("Need context and scope data and stuff");
                }

                DatexExpressionData::VariableAssignment(
                    _variable_assignment,
                ) => {
                    todo!("Need context and scope data and stuff");
                }

                DatexExpressionData::VariableAccess(_variable_access) => {
                    todo!("Need context and scope data and stuff");
                }

                e => panic!(
                    "Expression to instruction not implemented for {:?}",
                    e
                ),
            }
        })
    }
}

impl AstNodeToInstructions for CallableDeclaration {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &mut CompilationContext,
        metadata: &mut AstMetadata,
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

            let (instruction_block_data, new_scope) =
                crate::compiler::compile_child_realm_instructions(
                    compilation_context,
                    body,
                    injected_variable_count.unwrap(), // must be set by precompiler
                    metadata,
                    variables,
                )?;

            *compilation_context.scope = new_scope;

            yield Ok(RegularInstruction::CallableDeclaration(
                CallableDeclarationData {
                    signature: CallableSignatureData {
                        name: ShortTextData(signature.name.unwrap_or_default()),
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
            for (_, param) in signature.parameters {
                yield_all_ok!(param.to_instructions(compilation_context));
            }
            // add rest parameter type
            if let Some((_, param)) = signature.rest_parameter {
                yield_all_ok!(param.to_instructions(compilation_context));
            }
            // add return type
            if let Some(ty) = signature.return_type {
                yield_all_ok!(ty.to_instructions(compilation_context));
            }
            // add yield type
            if let Some(ty) = signature.yeet_type {
                yield_all_ok!(ty.to_instructions(compilation_context));
            }
        })
    }
}


impl ToInstructions for RangeDeclaration {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::range().into();
            yield_all!(self.start.to_instructions(visitor));
            yield_all!(self.end.to_instructions(visitor));
        })
    }
}

impl ToInstructions for ComparisonOperation {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::comparison_operation(self.operator)
                .into();
            yield_all!(self.left.to_instructions(visitor));
            yield_all!(self.right.to_instructions(visitor));
        })
    }
}

impl ToInstructions for UnboxAssignment {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            match self.operator {
                Some(operator) => match operator {
                    ModificationOperator::AddAssign => {
                        yield RegularInstruction::increment().into()
                    }
                    ModificationOperator::SubtractAssign => {
                        yield RegularInstruction::decrement().into()
                    }
                    _ => todo!("Generate x = x * z instructions;"),
                },
                None => {
                    yield RegularInstruction::set_shared_container_value()
                        .into()
                }
            };

            // compile assigned expression
            yield_all!(self.assigned_expression.to_instructions(visitor));

            // compile unbox expression
            yield_all!(self.unbox_expression.to_instructions(visitor));
        })
    }
}

impl ToInstructions for PropertyAssignment {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
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
                    yield RegularInstruction::set_entry_text(key.to_string())
                        .into();
                }

                DatexExpressionData::Integer(index)
                    if let Some(index) = index.as_u32() =>
                {
                    yield RegularInstruction::set_entry_index(index).into();
                }

                _ => {
                    for instruction in property.to_instructions(visitor) {
                        yield instruction;
                    }
                }
            }
            // compile assigned expression
            yield_all!(assigned_expression.to_instructions(visitor));

            // compile base expression
            yield_all!(base.to_instructions(visitor));
        })
    }
}

impl ToInstructions for UnaryOperation {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::unary_operation(self.operator).into();
            yield_all!(self.expression.to_instructions(visitor));
        })
    }
}

impl ToInstructions for Apply {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::apply(self.arguments.len() as u8).into();
            // compile arguments
            for argument in &self.arguments {
                yield_all!(argument.to_instructions(visitor));
            }
            // compile function expression
            yield_all!(self.base.to_instructions(visitor));
        })
    }
}

impl ToInstructions for InterfaceMethodCall {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            // TODO: replace with trait impls
            match self.method_name.as_str() {
                "append" => {
                    yield RegularInstruction::append_entry().into();

                    for argument in &self.arguments {
                        yield_all!(argument.to_instructions(visitor));
                    }
                }

                "clear" => {
                    yield RegularInstruction::clear().into();
                }

                "splice" => {
                    yield RegularInstruction::splice_dynamic().into();

                    for argument in &self.arguments {
                        yield_all!(argument.to_instructions(visitor));
                    }
                }

                _ => {
                    yield RegularInstruction::call_method(
                        self.method_name.clone(),
                        self.arguments.len() as u8,
                    )
                    .into();

                    for argument in &self.arguments {
                        yield_all!(argument.to_instructions(visitor));
                    }
                }
            }

            // compile target expression
            yield_all!(self.target.to_instructions(visitor));
        })
    }
}

impl ToInstructions for PropertyAccess {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            // depending on the key, handle different property accesses
            match self.property.data() {
                // simple text key if length fits in u8
                DatexExpressionData::Text(key) if key.len() <= 255 => {
                    yield RegularInstruction::get_entry_text(key.to_string())
                        .into();
                }
                // index access if integer fits in u32
                DatexExpressionData::Integer(index)
                    if let Some(index) = index.as_u32() =>
                {
                    yield RegularInstruction::get_entry_index(index).into();
                }
                _ => {
                    yield RegularInstruction::get_entry_dynamic().into();
                    yield_all!(self.property.to_instructions(visitor));
                }
            }

            // compile base expression
            yield_all!(self.base.to_instructions(visitor));
        })
    }
}

impl ToInstructions for GenericInstantiation {
    fn to_instructions<'a>(
        &'a self,
        _visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            // NOTE: might already be handled in type compilation
            todo!()
        })
    }
}

impl ToInstructions for RequestSharedRef {
    fn to_instructions<'a>(
        &'a self,
        _visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(core::iter::once(
            RegularInstruction::get_shared_ref(
                self.address.clone(),
                &self.mutability,
            )
            .into(),
        ))
    }
}

impl ToInstructions for List {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::list(self.items.len() as u32).into();
            for item in &self.items {
                yield_all!(item.to_instructions(visitor));
            }
        })
    }
}

impl ToInstructions for Map {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::map(self.entries.len() as u32).into();
            for (key, value) in &self.entries {
                match &*key.data {
                    // text -> insert key string
                    DatexExpressionData::Text(text) => {
                        if text.len() < 256 {
                            yield RegularInstruction::key_value_short_text(
                                text.as_string(),
                            )
                            .into();
                        } else {
                            yield RegularInstruction::key_value_dynamic()
                                .into();
                            yield RegularInstruction::text(text.as_string())
                                .into();
                        }
                    }
                    // other -> insert key as dynamic
                    _ => {
                        yield RegularInstruction::key_value_dynamic().into();
                        yield_all!(value.to_instructions(visitor));
                    }
                };

                // value
                yield_all!(value.to_instructions(visitor));
            }
        })
    }
}

impl ToInstructions for TagExpression {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::tagged_value(
                self.tag.clone(),
                self.expression.is_none(),
            )
            .into();
            if let Some(expression) = &self.expression {
                yield_all!(expression.to_instructions(visitor));
            }
        })
    }
}
impl ToInstructions for RootPropertyAccess {
    fn to_instructions<'a>(
        &'a self,
        _visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            let root_property = RootProperty::from_str(&self.property_name)
                .expect("invalid root property name");
            yield RegularInstruction::get_root_property(root_property).into();
        })
    }
}

impl ToInstructions for Unbox {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::unbox().into();
            yield_all!(self.expression.to_instructions(visitor));
        })
    }
}

impl ToInstructions for DeriveRef {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield_all!(self.expression.to_instructions(visitor));
        })
    }
}

impl ToInstructions for CreateShared {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            match self.mutability {
                SharedContainerMutability::Immutable => {
                    yield RegularInstruction::create_shared().into();
                }
                SharedContainerMutability::Mutable => {
                    yield RegularInstruction::create_shared_mut().into();
                }
            }

            yield_all!(self.expression.to_instructions(visitor));
        })
    }
}

impl ToInstructions for DeriveSharedRef {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            match self.mutability {
                ReferenceMutability::Immutable => {
                    yield RegularInstruction::derive_shared_reference().into();
                }
                ReferenceMutability::Mutable => {
                    yield RegularInstruction::derive_shared_reference_mut()
                        .into();
                }
            }

            yield_all!(self.expression.to_instructions(visitor));
        })
    }
}

impl ToInstructions for BinaryOperation {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::binary_operation(self.operator).into();
            yield_all!(self.left.to_instructions(visitor));
            yield_all!(self.right.to_instructions(visitor));
        })
    }
}
