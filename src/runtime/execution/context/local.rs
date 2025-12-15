use datex_core::runtime::execution::context::ExecutionContext;
use crate::stdlib::rc::Rc;
use crate::compiler::scope::CompilationScope;
use crate::runtime::execution::{ExecutionOptions, MemoryDump};
use crate::runtime::execution::execution_loop::state::{ExecutionLoopState, RuntimeExecutionState};
use crate::runtime::RuntimeInternal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionMode {
    /// A single execution of a program that is completely known at compile time.
    #[default]
    Static,
    /// An execution of a program that may be extended at runtime.
    /// This mode is used for REPLs and dynamic remote executions with persistent contexts.
    Unbounded,
}

#[derive(Debug, Default)]
pub struct LocalExecutionContext {
    #[cfg(feature = "compiler")]
    pub compile_scope: CompilationScope,
    pub loop_state: Option<ExecutionLoopState>,
    pub runtime: Option<Rc<RuntimeInternal>>,
    pub execution_options: ExecutionOptions,
    pub verbose: bool,
    pub execution_mode: ExecutionMode,
}

impl LocalExecutionContext {
    pub fn new(execution_mode: ExecutionMode) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(execution_mode),
            loop_state: None,
            runtime: None,
            execution_options: ExecutionOptions::default(),
            verbose: false,
            execution_mode,
        }
    }

    /// Creates a new local execution context with the given compile scope.
    pub fn debug(execution_mode: ExecutionMode) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(execution_mode),
            execution_options: ExecutionOptions { verbose: true },
            verbose: true,
            execution_mode,
            ..Default::default()
        }
    }

    pub fn debug_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        execution_mode: ExecutionMode,
    ) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(execution_mode),
            loop_state: None,
            runtime: Some(runtime_internal),
            execution_options: ExecutionOptions { verbose: true },
            verbose: true,
            execution_mode,
        }
    }

    pub fn new_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        execution_mode: ExecutionMode,
    ) -> Self {
        LocalExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(execution_mode),
            loop_state: None,
            runtime: Some(runtime_internal),
            execution_mode,
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
    pub fn local() -> Self {
        ExecutionContext::Local(LocalExecutionContext::new(ExecutionMode::Static))
    }

    /// Creates a new local execution context (can be used multiple times).
    pub fn local_unbounded() -> Self {
        ExecutionContext::Local(LocalExecutionContext::new(ExecutionMode::Unbounded))
    }

    /// Creates a new local execution context with a runtime.
    pub fn local_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        execution_mode: ExecutionMode,
    ) -> Self {
        ExecutionContext::Local(
            LocalExecutionContext::new_with_runtime_internal(
                runtime_internal,
                execution_mode,
            ),
        )
    }

    /// Creates a new local execution context with verbose mode enabled,
    /// providing more log outputs for debugging purposes.
    pub fn local_debug(execution_mode: ExecutionMode) -> Self {
        ExecutionContext::Local(LocalExecutionContext::debug(execution_mode))
    }

    /// Creates a new local execution context with verbose mode enabled and a runtime.
    pub fn local_debug_with_runtime_internal(
        runtime_internal: Rc<RuntimeInternal>,
        execution_mode: ExecutionMode,
    ) -> Self {
        ExecutionContext::Local(
            LocalExecutionContext::debug_with_runtime_internal(
                runtime_internal,
                execution_mode,
            ),
        )
    }

}