use serde::{Deserialize, Serializer};
use serde::de::Error;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;
use crate::dif::serde_context::SerdeContext;
use crate::dif::serialize_with_serde_context::SerializeWithSerdeContext;
use crate::libs::core::type_id::{CoreLibBaseTypeId, CoreLibVariantTypeId};
use crate::values::core_value::CoreValue;
use crate::prelude::*;

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
            CoreValue::Text(val) => Ok(val.0),
            _ => Err(D::Error::custom("Expected CoreValue::Text")),
        }
    }
}