use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::type_id::CoreLibBaseTypeId,
    prelude::*,
    values::core_values::decimal::Decimal,
};
use serde::{Deserialize, Serializer};
use serde::de::Error;
use crate::values::core_value::CoreValue;

impl SerializeWithSerdeContext for Decimal {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ctx.serialize_core_value(
            self,
            CoreLibBaseTypeId::Decimal.into(),
            serializer,
            false,
        )
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Decimal {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        match ctx.deserialize_core_value(deserializer)? {
            CoreValue::Decimal(td) => Ok(td),
            _ => Err(D::Error::custom("Expected CoreValue::Decimal")),
        }
    }
}
