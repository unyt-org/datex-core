use serde::{Deserialize, Serializer};
use serde::de::Error;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;
use crate::dif::serde_context::SerdeContext;
use crate::dif::serialize_with_serde_context::SerializeWithSerdeContext;
use crate::libs::core::type_id::CoreLibVariantTypeId;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::typed_decimal::{DecimalTypeVariant, TypedDecimal};

impl SerializeWithSerdeContext for f32 {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ctx.serialize_core_value(
            self,
            CoreLibVariantTypeId::Decimal(DecimalTypeVariant::F32).into(),
            serializer,
            false,
        )
    }
}


impl<'de> DeserializeWithSerdeContext<'de> for f32 {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        match ctx.deserialize_core_value(deserializer)? {
            CoreValue::TypedDecimal(TypedDecimal::F32(val)) => Ok(*val),
            _ => Err(D::Error::custom("Expected CoreValue::TypedDecimal(F32)")),
        }
    }
}


impl SerializeWithSerdeContext for f64 {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ctx.serialize_core_value(
            self,
            CoreLibVariantTypeId::Decimal(DecimalTypeVariant::F64).into(),
            serializer,
            true,
        )
    }
}


impl<'de> DeserializeWithSerdeContext<'de> for f64 {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        _ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        f64::deserialize(deserializer)
    }
}
