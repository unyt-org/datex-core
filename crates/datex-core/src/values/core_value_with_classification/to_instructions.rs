use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::{
        core_value_with_classification::CoreValueWithClassification,
        value::value_classification::ValueTag,
    },
};
use crate::core_compiler::shared_value_tracking::SharedValueTracking;
use crate::core_compiler::to_instructions::yield_all;

impl ToInstructions for CoreValueWithClassification {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            let classification = &self.classification;

            if let Some(entity_type) = &classification.entity_type {
                yield RegularInstruction::EntityValue(
                    entity_type.pointer_address(),
                )
                .into()
            }

            for _impl_address in &classification.impls {
                todo!(
                    "Compiling values with Impls classification is not yet implemented"
                )
            }

            if let Some(ValueTag { tag, is_empty }) = &classification.tag {
                yield RegularInstruction::tagged_value(tag.clone(), *is_empty)
                    .into();
                if *is_empty {
                    // early return, don't append null value; TODO: assert that value is actually null?
                    return;
                };
            }

            yield_all!(self.inner.to_instructions(visitor));
        })
    }
}
