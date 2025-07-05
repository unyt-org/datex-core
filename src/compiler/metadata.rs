#[derive(Debug, Clone, Default)]
pub struct CompileMetadata {
    is_outer_context: bool,
}

impl CompileMetadata {
    pub fn outer() -> Self {
        CompileMetadata {
            is_outer_context: true,
            ..CompileMetadata::default()
        }
    }
    pub fn is_outer_context(&self) -> bool {
        self.is_outer_context
    }
}
