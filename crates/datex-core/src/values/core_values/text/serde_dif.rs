use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::type_id::CoreLibBaseTypeId,
    prelude::*,
    values::core_values::text::Text,
};
use serde::{Deserialize, Serialize, Serializer};

impl Serialize for Text {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl SerializeWithSerdeContext for Text {
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

impl<'de> DeserializeWithSerdeContext<'de> for Text {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        _ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Text::String(s))
    }
}
