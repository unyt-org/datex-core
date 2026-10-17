use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    libs::core::core_lib_id::{CoreLibId, CoreLibIdIndex},
    prelude::*,
    values::core_value::CoreValue,
};
use crate::core_compiler::to_instructions::yield_all;

impl ToInstructions for CoreValue {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            match self {
                CoreValue::Uninitialized => unreachable!(),
                CoreValue::Null => yield RegularInstruction::null().into(),
                CoreValue::Boolean(boolean) => {
                    yield_all!(boolean.to_instructions(visitor));
                }
                CoreValue::Integer(integer) => {
                    yield_all!(integer.to_instructions(visitor));
                }
                CoreValue::TypedInteger(typed_integer) => {
                    yield_all!(typed_integer.to_instructions(visitor));
                }
                CoreValue::Decimal(decimal) => {
                    yield_all!(decimal.to_instructions(visitor));
                }
                CoreValue::TypedDecimal(typed_decimal) => {
                    yield_all!(typed_decimal.to_instructions(visitor));
                }
                CoreValue::Text(text) => {
                    yield_all!(text.to_instructions(visitor));
                }
                CoreValue::Endpoint(endpoint) => {
                    yield_all!(endpoint.to_instructions(visitor));
                }
                CoreValue::List(list) => {
                    yield_all!(list.to_instructions(visitor));
                }
                CoreValue::Map(map) => {
                    yield_all!(map.to_instructions(visitor));
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
                        yield_all!(ty.to_instructions(visitor));
                    }
                }
                CoreValue::EntityTypeDefinition(entity_type_definition) => {
                    yield_all!(entity_type_definition.to_instructions(visitor));
                }
                CoreValue::Callable(callable) => {
                    yield_all!(callable.to_instructions(visitor));
                }
                CoreValue::Range(range) => {
                    yield_all!(range.to_instructions(visitor));
                }
                CoreValue::Box(value_container) => {
                    yield RegularInstruction::boxed_value().into();
                    yield_all!(value_container.to_instructions(visitor));
                }
                CoreValue::Instant(instant) => {
                    yield_all!(instant.to_instructions(visitor));
                }
            }
        })
    }
}
