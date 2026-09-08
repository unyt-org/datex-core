use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    preludes::derive::RegularInstruction,
    values::value::{
        Value,
        value_classification::{ValueClassification, ValueTag},
    },
};

impl ToInstructions for Value {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            // append classified type information
            match &self.classification {
                ValueClassification::None => {
                    // no classification, just append the value
                }
                ValueClassification::Entity(entity_type) => {
                    yield RegularInstruction::EntityValue(
                        entity_type.pointer_address(),
                    )
                    .into()
                }
                ValueClassification::Impls(items) => todo!(
                    "Compiling values with Impls classification is not yet implemented"
                ),
                ValueClassification::Tag(ValueTag { tag, is_empty }) => {
                    yield RegularInstruction::tagged_value(
                        tag.clone(),
                        *is_empty,
                    )
                    .into();
                    if *is_empty {
                        // early return, don't append null value; TODO: assert that value is actually null?
                        return;
                    };
                }
            }

            // append inner instructions
            for instruction in self.inner.to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}
