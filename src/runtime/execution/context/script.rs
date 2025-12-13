use core::fmt::Display;
use crate::compiler::error::SpannedCompilerError;
use crate::runtime::execution::ExecutionError;

#[derive(Debug)]
pub enum ScriptExecutionError {
    #[cfg(feature = "compiler")]
    CompilerError(SpannedCompilerError),
    ExecutionError(ExecutionError),
}

#[cfg(feature = "compiler")]
impl From<SpannedCompilerError> for ScriptExecutionError {
    fn from(err: SpannedCompilerError) -> Self {
        ScriptExecutionError::CompilerError(err)
    }
}

impl From<ExecutionError> for ScriptExecutionError {
    fn from(err: ExecutionError) -> Self {
        ScriptExecutionError::ExecutionError(err)
    }
}

impl Display for ScriptExecutionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "compiler")]
            ScriptExecutionError::CompilerError(err) => {
                core::write!(f, "Compiler Error: {}", err)
            }
            ScriptExecutionError::ExecutionError(err) => {
                core::write!(f, "Execution Error: {}", err)
            }
        }
    }
}