#[derive(Debug, Default)]
pub struct InferExpressionTypeOptions {
    pub detailed_errors: bool,
    pub error_handling: ErrorHandling,
}

#[derive(Clone, Debug, Default)]
pub enum ErrorHandling {
    #[default]
    FailFast,
    Collect,
    CollectAndReturnType,
}
