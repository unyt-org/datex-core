use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
};

impl<K, V, S> ToInstructions for indexmap::IndexMap<K, V, S> {
    fn to_instructions<'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move { todo!() })
    }
}
