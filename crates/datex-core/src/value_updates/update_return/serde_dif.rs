use crate::{
    dif::serde_context::SerdeContext,
    prelude::*,
    dif::serialize_with_serde_context::SerializeWithSerdeContext,
    value_updates::UpdateReturn,
    values::value_container::ValueContainer,
};

use serde::{
    Serializer,
    ser::{SerializeSeq, SerializeStruct},
};
use crate::dif::value_with_serde_context::ValueWithSerdeContext;

impl SerializeWithSerdeContext for UpdateReturn {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;

        match self {
            UpdateReturn::None => {
                seq.serialize_element("none")?;
            }
            UpdateReturn::SingleValue(value) => {
                seq.serialize_element("single_value")?;
                seq.serialize_element(&ValueWithSerdeContext::new(
                    value,
                    ctx,
                ))?;
            }
            UpdateReturn::MultipleValues(values) => {
                seq.serialize_element("multiple_values")?;
                seq.serialize_element(&ValueWithSerdeContext::new(
                    values,
                    ctx,
                ))?;
            }
        }
        seq.end()
    }
}
