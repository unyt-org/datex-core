use core::cell::RefCell;
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::stdlib::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub verbose: bool,
}

/// Input required to execute a DXB program.
#[derive(Debug, Clone)]
pub struct ExecutionInput<'a> {
    /// Options for execution.
    pub options: ExecutionOptions,
    /// The DXB program body containing raw bytecode.
    pub dxb_body: &'a [u8],
    /// The execution should be ended after this run, no further executions will be done for this context.
    pub end_execution: bool,
    /// The runtime execution context that can persist across executions.
    pub context: Rc<RefCell<RuntimeExecutionState>>,
}

impl Default for ExecutionInput<'_> {
    fn default() -> Self {
        Self {
            options: ExecutionOptions::default(),
            dxb_body: &[],
            context: Rc::new(RefCell::new(RuntimeExecutionState::default())),
            end_execution: true,
        }
    }
}

impl<'a> ExecutionInput<'a> {
    pub fn new_with_dxb_and_options(
        dxb_body: &'a [u8],
        options: ExecutionOptions,
    ) -> Self {
        Self {
            options,
            dxb_body,
            context: Rc::new(RefCell::new(RuntimeExecutionState::default())),
            end_execution: true,
        }
    }
}