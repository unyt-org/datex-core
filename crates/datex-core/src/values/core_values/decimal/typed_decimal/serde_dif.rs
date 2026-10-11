use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::type_id::CoreLibVariantTypeId,
    values::{
        core_value::CoreValue,
        core_values::decimal::typed_decimal::TypedDecimal,
    },
};
use serde::{Deserializer, Serializer, de::Error};
impl SerializeWithSerdeContext for TypedDecimal {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ctx.serialize_core_value(
            self,
            CoreLibVariantTypeId::Decimal(self.variant()).into(),
            serializer,
            false,
        )
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for TypedDecimal {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        match ctx.deserialize_core_value(deserializer)? {
            CoreValue::TypedDecimal(td) => Ok(td),
            _ => Err(D::Error::custom("Expected CoreValue::TypedDecimal ")),
        }
    }
}
