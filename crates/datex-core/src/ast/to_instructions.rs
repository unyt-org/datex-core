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

impl ToInstructions for DatexExpressionData {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            match self {
                DatexExpressionData::Integer(integer) => {
                    yield RegularInstruction::Integer(integer.clone()).into()
                }
                DatexExpressionData::TypedInteger(typed_integer) => {
                    for instruction in typed_integer.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Instant(instant) => {
                    for instruction in instant.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Decimal(decimal) => {
                    for instruction in decimal.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::TypedDecimal(typed_decimal) => {
                    for instruction in typed_decimal.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Text(text) => {
                    for instruction in text.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Boolean(boolean) => {
                    for instruction in boolean.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Endpoint(endpoint) => {
                    for instruction in endpoint.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Null => {
                    yield RegularInstruction::null().into();
                }

                DatexExpressionData::List(list) => {
                    for instruction in list.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Map(map) => {
                    for instruction in map.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                // unary operations
                DatexExpressionData::UnaryOperation(unary_operation) => {
                    for instruction in unary_operation.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                // binary operations
                DatexExpressionData::BinaryOperation(binary_operation) => {
                    for instruction in binary_operation.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                // comparisons
                DatexExpressionData::ComparisonOperation(
                    comparison_operation,
                ) => {
                    for instruction in comparison_operation.to_instructions(ctx)
                    {
                        yield instruction;
                    }
                }

                // apply
                DatexExpressionData::Apply(apply) => {
                    for instruction in apply.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                // interface methods
                DatexExpressionData::InterfaceMethodCall(
                    interface_method_call,
                ) => {
                    for instruction in
                        interface_method_call.to_instructions(ctx)
                    {
                        yield instruction;
                    }
                }

                // property access
                DatexExpressionData::PropertyAccess(property_access) => {
                    for instruction in property_access.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::GenericInstantiation(
                    generic_instantiation,
                ) => {
                    for instruction in
                        generic_instantiation.to_instructions(ctx)
                    {
                        yield instruction;
                    }
                }

                DatexExpressionData::PropertyAssignment(
                    property_assignment,
                ) => {
                    for instruction in property_assignment.to_instructions(ctx)
                    {
                        yield instruction;
                    }
                }

                DatexExpressionData::RequestSharedRef(request_shared_ref) => {
                    for instruction in request_shared_ref.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::UnboxAssignment(unbox_assignment) => {
                    for instruction in unbox_assignment.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                // root property
                DatexExpressionData::RootPropertyAccess(
                    root_property_access,
                ) => {
                    for instruction in root_property_access.to_instructions(ctx)
                    {
                        yield instruction;
                    }
                }

                // refs
                DatexExpressionData::DeriveRef(derive_ref) => {
                    for instruction in derive_ref.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                // shared refs
                DatexExpressionData::DeriveSharedRef(derive_shared_ref) => {
                    for instruction in derive_shared_ref.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                // shared values
                DatexExpressionData::CreateShared(create_shared) => {
                    for instruction in create_shared.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::TypeExpression(type_expression) => {
                    yield RegularInstruction::TypeExpression.into();
                    for instruction in type_expression.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Range(range_dec) => {
                    for instruction in range_dec.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Unbox(unbox) => {
                    for instruction in unbox.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::Tag(tag_expression) => {
                    for instruction in tag_expression.to_instructions(ctx) {
                        yield instruction;
                    }
                }

                DatexExpressionData::ResolveCoreLibId(core_lib_id) => {
                    yield RegularInstruction::get_core_lib_value(
                        (*core_lib_id).into(),
                    )
                    .into();
                }

                DatexExpressionData::CallableDeclaration(
                    _callable_declaration,
                ) => {
                    todo!("Need context and scope data and stuff");
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

impl ToInstructions for RangeDeclaration {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::range().into();
            for instruction in self.start.to_instructions(ctx) {
                yield instruction;
            }
            for instruction in self.end.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for ComparisonOperation {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::comparison_operation(self.operator)
                .into();
            for instruction in self.left.to_instructions(ctx) {
                yield instruction;
            }
            for instruction in self.right.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for UnboxAssignment {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
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
            for instruction in self.assigned_expression.to_instructions(ctx) {
                yield instruction;
            }

            // compile unbox expression
            for instruction in self.unbox_expression.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for PropertyAssignment {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
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
                    if key.0.len() <= u8::MAX as usize =>
                {
                    yield RegularInstruction::set_entry_text(key.0.clone())
                        .into();
                }

                DatexExpressionData::Integer(index)
                    if let Some(index) = index.as_u32() =>
                {
                    yield RegularInstruction::set_entry_index(index).into();
                }

                _ => {
                    for instruction in property.to_instructions(ctx) {
                        yield instruction;
                    }
                }
            }
            // compile assigned expression
            for instruction in assigned_expression.to_instructions(ctx) {
                yield instruction;
            }

            // compile base expression
            for instruction in base.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for UnaryOperation {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::unary_operation(self.operator).into();
            for instruction in self.expression.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for Apply {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::apply(self.arguments.len() as u8).into();
            // compile arguments
            for argument in &self.arguments {
                for instruction in argument.to_instructions(ctx) {
                    yield instruction;
                }
            }
            // compile function expression
            for instruction in self.base.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for InterfaceMethodCall {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            // TODO: replace with trait impls
            match self.method_name.as_str() {
                "append" => {
                    yield RegularInstruction::append_entry().into();

                    for argument in &self.arguments {
                        for instruction in argument.to_instructions(ctx) {
                            yield instruction;
                        }
                    }
                }

                "clear" => {
                    yield RegularInstruction::clear().into();
                }

                "splice" => {
                    yield RegularInstruction::splice_dynamic().into();

                    for argument in &self.arguments {
                        for instruction in argument.to_instructions(ctx) {
                            yield instruction;
                        }
                    }
                }

                _ => {
                    yield RegularInstruction::call_method(
                        self.method_name.clone(),
                        self.arguments.len() as u8,
                    )
                    .into();

                    for argument in &self.arguments {
                        for instruction in argument.to_instructions(ctx) {
                            yield instruction;
                        }
                    }
                }
            }

            // compile target expression
            for instruction in self.target.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for PropertyAccess {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            // depending on the key, handle different property accesses
            match self.property.data() {
                // simple text key if length fits in u8
                DatexExpressionData::Text(key) if key.0.len() <= 255 => {
                    yield RegularInstruction::get_entry_text(key.0.clone())
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
                    for instruction in self.property.to_instructions(ctx) {
                        yield instruction;
                    }
                }
            }

            // compile base expression
            for instruction in self.base.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for GenericInstantiation {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            // NOTE: might already be handled in type compilation
            todo!()
        })
    }
}

impl ToInstructions for RequestSharedRef {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
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
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::list(self.items.len() as u32).into();
            for item in &self.items {
                for instruction in item.to_instructions(ctx) {
                    yield instruction;
                }
            }
        })
    }
}

impl ToInstructions for Map {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::map(self.entries.len() as u32).into();
            for (key, value) in &self.entries {
                match &*key.data {
                    // text -> insert key string
                    DatexExpressionData::Text(text) => {
                        if text.len() < 256 {
                            yield RegularInstruction::key_value_short_text(
                                text.0.clone(),
                            )
                            .into();
                        } else {
                            yield RegularInstruction::key_value_dynamic()
                                .into();
                            yield RegularInstruction::text(text.0.clone())
                                .into();
                        }
                    }
                    // other -> insert key as dynamic
                    _ => {
                        yield RegularInstruction::key_value_dynamic().into();
                        for instruction in key.to_instructions(ctx) {
                            yield instruction;
                        }
                    }
                };

                // value
                for instruction in value.to_instructions(ctx) {
                    yield instruction;
                }
            }
        })
    }
}

impl ToInstructions for TagExpression {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::tagged_value(
                self.tag.clone(),
                self.expression.is_none(),
            )
            .into();
            if let Some(expression) = &self.expression {
                for instruction in expression.to_instructions(ctx) {
                    yield instruction;
                }
            }
        })
    }
}
impl ToInstructions for RootPropertyAccess {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            let root_property = RootProperty::from_str(&self.property_name)
                .expect("invalid root property name");
            yield RegularInstruction::get_root_property(root_property).into();
        })
    }
}

impl ToInstructions for Unbox {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::unbox().into();
            for instruction in self.expression.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for DeriveRef {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            for instruction in self.expression.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for CreateShared {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
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

            for instruction in self.expression.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for DeriveSharedRef {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
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

            for instruction in self.expression.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for BinaryOperation {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::binary_operation(self.operator).into();
            for instruction in self.left.to_instructions(ctx) {
                yield instruction;
            }
            for instruction in self.right.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}
