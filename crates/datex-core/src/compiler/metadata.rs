#[derive(Debug, Clone, Default, Copy)]
pub struct CompileContextMetadata {
    is_outer_context: bool,
}

impl CompileContextMetadata {
    pub fn outer() -> Self {
        CompileContextMetadata {
            is_outer_context: true,
        }
    }
    pub fn is_outer_context(&self) -> bool {
        self.is_outer_context
    }
}
