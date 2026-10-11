use crate::{
    prelude::*,
    runtime::Runtime,
    traits::apply::{Apply, ApplyArgument, ApplyError},
    values::{value::Value, value_container::ValueContainer},
};

impl Apply for Value {
    fn try_apply_sync(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        match self {
            Value::Core(core_value) => core_value.try_apply_sync(runtime, args),
            _ => Err(ApplyError::UnsupportedApply),
        }
    }

    async fn try_apply_async(
        &self,
        runtime: &Runtime,
        args: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        match self {
            Value::Core(core_value) => {
                core_value.try_apply_async(runtime, args).await
            }
            _ => Err(ApplyError::UnsupportedApply),
        }
    }
}
