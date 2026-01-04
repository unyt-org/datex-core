use core::fmt::{Display, Formatter};

use realhydroper_lsp::lsp_types::Range;

use crate::compiler::error::CompilerError;

#[derive(Debug)]
pub struct SpannedLSPCompilerError {
    pub error: CompilerError,
    pub span: Range,
}

impl SpannedLSPCompilerError {
    pub fn new_with_span(
        error: CompilerError,
        span: Range,
    ) -> SpannedLSPCompilerError {
        SpannedLSPCompilerError { error, span }
    }
}

impl Display for SpannedLSPCompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        core::write!(
            f,
            "{} ({}:{}..{}:{})",
            self.error,
            self.span.start.line,
            self.span.start.character,
            self.span.end.line,
            self.span.end.character
        )
    }
}
