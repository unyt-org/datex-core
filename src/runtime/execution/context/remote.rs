use crate::compiler::scope::CompilationScope;
use crate::global::dxb_block::OutgoingContextId;
use crate::runtime::execution::context::{ExecutionContext, ExecutionMode};
use crate::values::core_values::endpoint::Endpoint;

#[derive(Debug, Clone, Default)]
pub struct RemoteExecutionContext {
    #[cfg(feature = "compiler")]
    pub compile_scope: CompilationScope,
    pub endpoint: Endpoint,
    pub context_id: Option<OutgoingContextId>,
    pub execution_mode: ExecutionMode,
}

impl RemoteExecutionContext {
    /// Creates a new remote execution context with the given endpoint.
    pub fn new(endpoint: impl Into<Endpoint>, execution_mode: ExecutionMode) -> Self {
        RemoteExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(execution_mode),
            endpoint: endpoint.into(),
            context_id: None,
            execution_mode,
        }
    }
}


impl ExecutionContext {
    pub fn remote(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote(RemoteExecutionContext::new(endpoint, ExecutionMode::Static))
    }

    pub fn remote_unbounded(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote(RemoteExecutionContext::new(endpoint, ExecutionMode::Unbounded))
    }
}