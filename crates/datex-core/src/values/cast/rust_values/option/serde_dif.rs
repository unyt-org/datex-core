use core::fmt::Formatter;
use erased_serde::__private::serde::de::{Error, Visitor};
use crate::dif::{
    deserialize_with_serde_context::{
        DeserializeWithSerdeContext, DeserializeWithSerdeContextDyn,
    },
    serde_context::SerdeContext,
    serialize_with_serde_context::{
        SerializeWithSerdeContext, SerializeWithSerdeContextDyn,
    },
};
use serde::{Serialize, Serializer};
use serde::de::DeserializeSeed;
use crate::dif::deserialize_serde_context::{DeserializeSerdeContext, ErasedSeed};

impl<T: SerializeWithSerdeContextDyn> SerializeWithSerdeContext for Option<T> {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Some(value) => value.with_ctx(ctx).serialize(serializer),
            None => serializer.serialize_none(),
        }
    }
}

impl<'de, T: DeserializeWithSerdeContextDyn> DeserializeWithSerdeContext<'de>
    for Option<T>
{
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DeserializeSerdeContext::<Option<T>>::new(ctx))
    }
}

impl<'de, 'a, 'ctx, T> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, Option<T>>
where
    T: DeserializeWithSerdeContextDyn,
{
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        formatter.write_str("an optional value")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let seed = ErasedSeed::<T>::new(self.ctx);
        let value = seed.deserialize(deserializer)?;
        Ok(Some(value))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(None)
    }
}