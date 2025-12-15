use core::cell::RefCell;
use crate::parser::body::DXBParserError;
use crate::runtime::execution::execution_loop::{execution_loop, ExternalExecutionInterrupt, InterruptProvider};
use crate::runtime::execution::execution_loop::state::{ExecutionLoopState, RuntimeExecutionState};
use crate::runtime::execution::ExecutionError;
use crate::runtime::RuntimeInternal;
use crate::stdlib::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub verbose: bool,
}

/// Input required to execute a DXB program.
#[derive(Debug, Default)]
pub struct ExecutionInput<'a> {
    /// Options for execution.
    pub options: ExecutionOptions,
    /// The DXB program body containing raw bytecode.
    pub dxb_body: &'a [u8],
    /// For persisting execution state across multiple executions (e.g., for REPL scenarios).
    pub loop_state: Option<ExecutionLoopState>,
    pub runtime: Option<Rc<RuntimeInternal>>,
}

impl<'a> ExecutionInput<'a> {
    pub fn new(
        dxb_body: &'a [u8],
        options: ExecutionOptions,
        runtime: Option<Rc<RuntimeInternal>>,
    ) -> Self {
        Self {
            options,
            dxb_body,
            loop_state: None,
            runtime,
        }
    }

    pub fn execution_loop(
        mut self,
        interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
    ) -> impl Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>> {
        gen move {
            // use execution iterator if one already exists from previous execution
            let mut loop_state = if let Some(existing_loop_state) = self.loop_state.take() {
                // update dxb so that instruction iterator can continue with next instructions
                *existing_loop_state.dxb_body.borrow_mut() = self.dxb_body.to_vec();
                existing_loop_state
            }
            // otherwise start a new execution loop
            else {
                let state = RuntimeExecutionState::default();
                // TODO: optimize, don't clone the whole DXB body every time here
                let dxb_rc = Rc::new(RefCell::new(self.dxb_body.to_vec()));
                ExecutionLoopState {
                    dxb_body: dxb_rc.clone(),
                    iterator: Box::new(execution_loop(state, dxb_rc, interrupt_provider.clone()))
                }
            };

            // proxy the iterator, storing it back into state if interrupted to await more instructions
            loop {
                let item = loop_state.iterator.next();
                if item.is_none() {
                    break;
                }
                let item = item.unwrap();

                match item {
                    Err(ExecutionError::IntermediateResultWithState(intermediate_result, _)) => {
                        return yield Err(ExecutionError::IntermediateResultWithState(intermediate_result, Some(loop_state)));
                    }
                    _ => yield item
                }
            }

        }
    }
}