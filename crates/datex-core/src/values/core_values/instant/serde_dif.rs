use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::type_id::CoreLibBaseTypeId,
    values::{core_value::CoreValue, core_values::instant::Instant},
};
use erased_serde::__private::serde::Serializer;
use serde::{Deserialize, Serialize, de::Error};

impl Serialize for Instant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer
            .serialize_i128(self.0)
            .map_err(serde::ser::Error::custom)
    }
}

impl<'de> Deserialize<'de> for Instant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let i = i128::deserialize(deserializer)?;
        Ok(Instant(i))
    }
}

/*
TODO: handle
   ctx.serialize_core_value(
            self,
            CoreLibBaseTypeId::Endpoint.into(),
            &ValueClassification::default(),
            serializer,
            false,
        )
in macro
 */

impl SerializeWithSerdeContext for Instant {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ctx.serialize_core_value(
            self,
            CoreLibBaseTypeId::Instant.into(),
            serializer,
            false,
        )
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Instant {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ctx.deserialize_core_value(deserializer)? {
            CoreValue::Instant(instant) => Ok(instant),
            _ => Err(D::Error::custom("Expected CoreValue::Instant")),
        }
    }
}
