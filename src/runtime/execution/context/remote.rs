use crate::compiler::scope::CompilationScope;
use crate::global::dxb_block::OutgoingContextId;
use crate::runtime::execution::context::ExecutionContext;
use crate::values::core_values::endpoint::Endpoint;

#[derive(Debug, Clone, Default)]
pub struct RemoteExecutionContext {
    #[cfg(feature = "compiler")]
    pub compile_scope: CompilationScope,
    pub endpoint: Endpoint,
    pub context_id: Option<OutgoingContextId>,
}

impl RemoteExecutionContext {
    /// Creates a new remote execution context with the given endpoint.
    pub fn new(endpoint: impl Into<Endpoint>, once: bool) -> Self {
        RemoteExecutionContext {
            #[cfg(feature = "compiler")]
            compile_scope: CompilationScope::new(once),
            endpoint: endpoint.into(),
            context_id: None,
        }
    }
}


impl ExecutionContext {
    pub fn remote_once(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote(RemoteExecutionContext::new(endpoint, true))
    }

    pub fn remote(endpoint: impl Into<Endpoint>) -> Self {
        ExecutionContext::Remote(RemoteExecutionContext::new(endpoint, false))
    }
}