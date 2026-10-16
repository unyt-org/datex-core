use crate::core_compiler::instruction_input::InstructionInput;
use crate::core_compiler::to_instructions::{ToInstructions};
use crate::core_compiler::value_visitor::ValueVisitor;
use crate::instruction::Instruction;

impl ToInstructions for InstructionInput {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        match self {
            InstructionInput::Instruction(instruction) => Box::new(core::iter::once(*instruction)),
            InstructionInput::ValueContainer(value_container) => value_container.to_instructions(visitor),
        }
    }
}

impl ToInstructions for Vec<InstructionInput> {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(self.iter().flat_map(move |input| input.to_instructions(visitor)))
    }
}