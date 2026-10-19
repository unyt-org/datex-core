use core::time::Duration;

use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
};

// impl IntoRegularInstruction for Instant {
//     fn into_regular_instruction(&self) -> RegularInstruction {
//         // TODO: Implement the conversion from Instant to RegularInstruction
//         unimplemented!()
//     }
// }

impl ToInstructions for Duration {
    fn to_bytecode_item<'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move { todo!() })
    }
}
// impl ToInstructionsDyn for Duration {
//     fn to_instructions_dyn<'a, 'ctx>(
//         &'a self,
//         ctx: &'a mut (dyn ValueVisitor + 'ctx),
//     ) -> Box<dyn Iterator<Item = crate::instruction::CompileItem> + 'a>
//     where
//         'ctx: 'a,
//     {
//         Box::new(self.to_instructions(ctx))
//     }
// }
