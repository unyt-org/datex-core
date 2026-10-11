use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::type_id::CoreLibVariantTypeId,
    values::{
        core_value::CoreValue,
        core_values::integer::typed_integer::TypedInteger,
    },
};
use serde::{Serialize, Serializer, de::Error};
impl Serialize for TypedInteger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TypedInteger::IBig(v) => v.serialize(serializer),
            TypedInteger::I8(v) => v.serialize(serializer),
            TypedInteger::I16(v) => v.serialize(serializer),
            TypedInteger::I32(v) => v.serialize(serializer),
            TypedInteger::I64(v) => v.serialize(serializer),
            TypedInteger::I128(v) => v.serialize(serializer),
            TypedInteger::U8(v) => v.serialize(serializer),
            TypedInteger::U16(v) => v.serialize(serializer),
            TypedInteger::U32(v) => v.serialize(serializer),
            TypedInteger::U64(v) => v.serialize(serializer),
            TypedInteger::U128(v) => v.serialize(serializer),
        }
    }
}

impl SerializeWithSerdeContext for TypedInteger {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ctx.serialize_core_value(
            self,
            CoreLibVariantTypeId::Integer(self.variant()).into(),
            serializer,
            false,
        )
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for TypedInteger {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        match ctx.deserialize_core_value(deserializer)? {
            CoreValue::TypedInteger(ti) => Ok(ti),
            _ => Err(D::Error::custom("Expected CoreValue::TypedInteger ")),
        }
    }
}
