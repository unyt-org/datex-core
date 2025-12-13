use core::cell::RefCell;
use datex_core::runtime::execution::context::ExecutionContext;
use crate::stdlib::rc::Rc;
use crate::compiler::scope::CompilationScope;
use crate::runtime::execution::{ExecutionOptions, MemoryDump};
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::RuntimeInternal;

#[derive(Debug, Clone, Default)]
pub struct LocalExecutionContext {
    #[cfg(feature = "compiler")]
    pub compile_scope: CompilationScope,
    pub runtime_execution_state: Rc<RefCell<RuntimeExecutionState>>,
    pub execution_options: ExecutionOptions,
    pub verbose: bool,
}

impl LocalExecutionContext {
    pub fn new(once: bool) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(once),
            runtime_execution_state: Rc::new(RefCell::new(
                RuntimeExecutionState::default(),
            )),
            execution_options: ExecutionOptions::default(),
            verbose: false,
        }
    }

    /// Creates a new local execution context with the given compile scope.
    pub fn debug(once: bool) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(once),
            execution_options: ExecutionOptions { verbose: true },
            verbose: true,
            ..Default::default()
        }
    }

    pub fn debug_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(once),
            runtime_execution_state: Rc::new(RefCell::new(
                RuntimeExecutionState::new(runtime_internal),
            )),
            execution_options: ExecutionOptions { verbose: true },
            verbose: true,
        }
    }

    pub fn new_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(once),
            runtime_execution_state: Rc::new(RefCell::new(
                RuntimeExecutionState::new(runtime_internal),
            )),
            ..Default::default()
        }
    }

    pub fn set_runtime_internal(
        &mut self,
        runtime_internal: Rc<RuntimeInternal>,
    ) {
        self.runtime_execution_state
            .borrow_mut()
            .set_runtime_internal(runtime_internal);
    }

    /// Returns a memory dump of the current state of the execution context.
    pub fn memory_dump(&self) -> MemoryDump {
        self.runtime_execution_state.borrow().memory_dump()
    }
}


impl ExecutionContext {
    /// Creates a new local execution context (can only be used once).
    pub fn local_once() -> Self {
        ExecutionContext::Local(LocalExecutionContext::new(true))
    }

    /// Creates a new local execution context (can be used multiple times).
    pub fn local() -> Self {
        ExecutionContext::Local(LocalExecutionContext::new(false))
    }

    /// Creates a new local execution context with a runtime.
    pub fn local_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        ExecutionContext::Local(
            LocalExecutionContext::new_with_runtime_internal(
                runtime_internal,
                once,
            ),
        )
    }

    /// Creates a new local execution context with verbose mode enabled,
    /// providing more log outputs for debugging purposes.
    pub fn local_debug(once: bool) -> Self {
        ExecutionContext::Local(LocalExecutionContext::debug(once))
    }

    /// Creates a new local execution context with verbose mode enabled and a runtime.
    pub fn local_debug_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        once: bool,
    ) -> Self {
        ExecutionContext::Local(
            LocalExecutionContext::debug_with_runtime_internal(
                runtime_internal,
                once,
            ),
        )
    }

}