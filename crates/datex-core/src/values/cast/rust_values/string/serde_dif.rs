use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::type_id::{CoreLibBaseTypeId, CoreLibVariantTypeId},
    prelude::*,
    values::core_value::CoreValue,
};
use serde::{Deserialize, Serializer, de::Error};

impl SerializeWithSerdeContext for String {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ctx.serialize_core_value(
            self,
            CoreLibBaseTypeId::Text.into(),
            serializer,
            true,
        )
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for String {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        match ctx.deserialize_core_value(deserializer)? {
            CoreValue::Text(val) => Ok(val.as_string()),
            _ => Err(D::Error::custom("Expected CoreValue::Text")),
        }
    }
}
