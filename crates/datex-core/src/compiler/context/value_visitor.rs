use crate::compiler::context::CompilationContext;
use crate::core_compiler::value_visitor::ValueVisitor;
use crate::instruction::Instruction;
use crate::shared_values::{SharedContainer};
use crate::prelude::*;

impl<'ctx> ValueVisitor for CompilationContext<'ctx> {
    fn visit_shared_container<'a>(
        &'a mut self,
        shared_container: &'a SharedContainer
    ) -> Box<dyn Iterator<Item = Instruction> + 'a> {
        // Delegate the visit to the shared value tracking system
        self.core_context.shared_value_tracking.get_mut().visit_shared_container(shared_container)
    }
}