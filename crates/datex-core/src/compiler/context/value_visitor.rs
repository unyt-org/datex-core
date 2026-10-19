use crate::{
    compiler::context::CompilationContext,
    core_compiler::value_visitor::ValueVisitor, instruction::Instruction,
    prelude::*, shared_values::SharedContainer,
};

impl<'ctx> ValueVisitor for CompilationContext<'ctx> {
    fn visit_shared_container<'a>(
        &'a mut self,
        shared_container: &'a SharedContainer,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        // Delegate the visit to the shared value tracking system
        self.core_context
            .shared_value_tracking
            .get_mut()
            .visit_shared_container(shared_container)
    }
}
