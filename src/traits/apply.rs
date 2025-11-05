use crate::runtime::execution::ExecutionError;
use crate::values::value_container::ValueContainer;
use core::prelude::rust_2024::*;

// TODO #351: return ApplyErrors including call stack information (or store call stack directly in ExecutionError)
pub trait Apply {
    /// Applies multiple ValueContainer arguments to self
    fn apply(
        &self,
        args: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ExecutionError>;
    /// Applies a single ValueContainer argument to self
    fn apply_single(
        &self,
        arg: &ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError>;
}
