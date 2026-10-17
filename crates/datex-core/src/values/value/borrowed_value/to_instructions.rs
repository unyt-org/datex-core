use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::value::borrowed_value::BorrowedValue,
};
use itertools::Itertools;

impl ToInstructions for BorrowedValue<'_> {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        match self {
            BorrowedValue::Native(native) => native.to_instructions(visitor),
            BorrowedValue::Core(core) => {
                let core = core.clone_to_core_value_with_classification();
                // FIXME: get this to work without collect
                Box::new(core.to_instructions(visitor).collect_vec().into_iter())
            }
        }
    }
}
