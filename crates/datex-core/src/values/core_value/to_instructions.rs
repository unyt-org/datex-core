use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    libs::core::core_lib_id::{CoreLibId, CoreLibIdIndex},
    prelude::*,
    preludes::derive::RegularInstruction,
    values::core_value::CoreValue,
};

impl ToInstructions for CoreValue {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            match self {
                CoreValue::Uninitialized => unreachable!(),
                CoreValue::Null => yield RegularInstruction::null().into(),
                CoreValue::Boolean(boolean) => {
                    for instruction in boolean.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Integer(integer) => {
                    for instruction in integer.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::TypedInteger(typed_integer) => {
                    for instruction in typed_integer.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Decimal(decimal) => {
                    for instruction in decimal.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::TypedDecimal(typed_decimal) => {
                    for instruction in typed_decimal.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Text(text) => {
                    for instruction in text.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Endpoint(endpoint) => {
                    for instruction in endpoint.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::List(list) => {
                    for instruction in list.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Map(map) => {
                    for instruction in map.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Type(ty) => {
                    if let Some(core_id) = ty.try_as_core_lib_type() {
                        // Shortcut for core library types
                        yield RegularInstruction::get_core_lib_value(
                            CoreLibIdIndex::from(CoreLibId::Type(core_id)),
                        )
                        .into()
                    } else {
                        // Switch to type space
                        yield RegularInstruction::type_expression().into();
                        for instruction in ty.to_instructions(ctx) {
                            yield instruction;
                        }
                    }
                }
                CoreValue::EntityTypeDefinition(entity_type_definition) => {
                    for instruction in
                        entity_type_definition.to_instructions(ctx)
                    {
                        yield instruction;
                    }
                }
                CoreValue::Callable(callable) => {
                    for instruction in callable.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Range(range) => {
                    for instruction in range.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Box(value_container) => {
                    for instruction in value_container.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                CoreValue::Native(native_core_value) => {
                    for instruction in native_core_value.to_instructions(ctx) {
                        yield instruction;
                    }
                }
            }
        })
    }
}
