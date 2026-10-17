use crate::compiler::scope::CompilationScope;
use crate::core_compiler::core_compilation_context::DXBWithSharedValues;

/// result of a compilation, including the compiled DXB and the scope of the compile context
#[derive(Debug)]
pub struct CompilationResult {
    pub dxb_with_shared_values: DXBWithSharedValues,
    pub scope: CompilationScope,
    pub has_non_static_value: bool,
}
