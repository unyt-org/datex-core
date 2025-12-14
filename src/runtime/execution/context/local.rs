use datex_core::runtime::execution::context::ExecutionContext;
use crate::stdlib::rc::Rc;
use crate::compiler::scope::CompilationScope;
use crate::runtime::execution::{ExecutionOptions, MemoryDump};
use crate::runtime::execution::execution_loop::state::{ExecutionLoopState, RuntimeExecutionState};
use crate::runtime::RuntimeInternal;

#[derive(Debug, Default)]
pub struct LocalExecutionContext {
    #[cfg(feature = "compiler")]
    pub compile_scope: CompilationScope,
    pub loop_state: Option<ExecutionLoopState>,
    pub runtime: Option<Rc<RuntimeInternal>>,
    pub execution_options: ExecutionOptions,
    pub verbose: bool,
}

impl LocalExecutionContext {
    pub fn new(once: bool) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(once),
            loop_state: None,
            runtime: None,
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
            loop_state: None,
            runtime: Some(runtime_internal),
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
            loop_state: None,
            runtime: Some(runtime_internal),
            ..Default::default()
        }
    }

    pub fn set_runtime_internal(
        &mut self,
        runtime_internal: Rc<RuntimeInternal>,
    ) {
        self.runtime = Some(runtime_internal);
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