#[derive(Debug, Clone, Default)]
pub struct PrecompilerOptions {
    /// If enabled, all collected errors as well as the RichAst
    /// are returned if one or multiple errors occurred.
    /// Otherwise, only the first error is returned (fast failing)
    pub detailed_errors: bool,
}
