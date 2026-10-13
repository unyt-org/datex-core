use crate::dif::{
    deserialize_serde_context::{DeserializeSerdeContext, ErasedSeed},
    deserialize_with_serde_context::{
        DeserializeWithSerdeContext, DeserializeWithSerdeContextDyn,
    },
    serde_context::SerdeContext,
    serialize_with_serde_context::{
        SerializeWithSerdeContext, SerializeWithSerdeContextDyn,
    },
};
use core::fmt::Formatter;
use serde::{de::{DeserializeSeed, Error, Visitor}, Deserializer, Serialize, Serializer};
use serde::de::{EnumAccess, IntoDeserializer, MapAccess, SeqAccess};
use serde::de::value::{EnumAccessDeserializer, MapAccessDeserializer, SeqAccessDeserializer};
use crate::prelude::*;

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
        deserializer
            .deserialize_any(DeserializeSerdeContext::<Option<T>>::new(ctx))
    }
}

/// Forwards a value-carrying visit method through `IntoDeserializer`.
macro_rules! forward_via_into_deserializer {
    ($($method:ident: $ty:ty),* $(,)?) => {$(
        fn $method<E: Error>(self, v: $ty) -> Result<Self::Value, E> {
            self.some(IntoDeserializer::<'_, E>::into_deserializer(v))
        }
    )*};
}

impl<'a, 'ctx, T> DeserializeSerdeContext<'a, 'ctx, Option<T>>
where
    T: DeserializeWithSerdeContextDyn,
{
    /// Deserialize the inner `T` from any deserializer and wrap it in `Some`.
    fn some<'de, D: Deserializer<'de>>(self, d: D) -> Result<Option<T>, D::Error> {
        ErasedSeed::<T>::new(self.ctx).deserialize(d).map(Some)
    }
}

impl<'de, 'a, 'ctx, T> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, Option<T>>
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
        D: Deserializer<'de>,
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

    // forward all the visit methods for the primitive types to the `some` method, which will deserialize them into `T` and wrap in `Some`.
    forward_via_into_deserializer! {
        visit_bool: bool,
        visit_i8: i8, visit_i16: i16, visit_i32: i32, visit_i64: i64, visit_i128: i128,
        visit_u8: u8, visit_u16: u16, visit_u32: u32, visit_u64: u64, visit_u128: u128,
        visit_f32: f32, visit_f64: f64,
        visit_char: char,
        visit_str: &str,
        visit_string: String,
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        self.some(SeqAccessDeserializer::new(seq))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        self.some(MapAccessDeserializer::new(map))
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        self.some(EnumAccessDeserializer::new(data))
    }
}
