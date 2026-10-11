use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    prelude::*,
    values::core_values::callable::Callable,
};
use core::fmt;
use serde::{
    Deserializer, Serializer,
    de::{DeserializeSeed, SeqAccess, Visitor},
    ser::SerializeTuple,
};

impl SerializeWithSerdeContext for Callable {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // put callable into cache
        let hash = ctx
            .shared_container_cache
            .borrow_mut()
            .store_callable(self.clone());
        let mut data = serializer.serialize_tuple(3)?;

        // store hash
        data.serialize_element(&hash.to_string())?;

        // store name
        data.serialize_element(&self.name)?;

        // async bool
        data.serialize_element(&self.signature.requires_async)?;

        data.end()
        // todo: also store function signature information
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Callable {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Callable, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(DeserializeSerdeContext::<Callable>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, Callable>
{
    type Value = Callable;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(
            "either an object with string keys or a sequence of [key, value] entries",
        )
    }
    fn visit_seq<A>(mut self, mut seq: A) -> Result<Callable, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let hash: String = seq.next_element()?.ok_or_else(|| {
            serde::de::Error::custom("Expected hash string as first element")
        })?;
        let hash_u64 = hash.parse::<u64>().map_err(|_| {
            serde::de::Error::custom("Failed to parse hash string to u64")
        })?;

        let _name: Option<Option<String>> = seq.next_element()?;
        let _requires_async: Option<bool> = seq.next_element()?;

        let callable = self
            .ctx
            .shared_container_cache
            .borrow_mut()
            .get_callable(hash_u64)
            .ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "Callable with hash {} not found in cache",
                    hash_u64
                ))
            })?;

        Ok(callable)
    }
}
