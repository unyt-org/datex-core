use crate::{
    dif::{
        deserialize_serde_context::{DeserializeSerdeContext, ErasedSeed},
        deserialize_with_serde_context::{
            DeserializeWithSerdeContext, DeserializeWithSerdeContextDyn,
        },
        serde_context::SerdeContext,
        serialize_with_serde_context::{
            SerializeWithSerdeContext, SerializeWithSerdeContextDyn,
        },
    },
    prelude::*,
};
use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
};

impl<T: SerializeWithSerdeContextDyn> SerializeWithSerdeContext for Vec<T> {
    fn serialize_with_ctx<S: serde::Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for value in self {
            seq.serialize_element(&*value.with_ctx(ctx))?;
        }
        seq.end()
    }
}

impl<'de, T: DeserializeWithSerdeContextDyn> DeserializeWithSerdeContext<'de>
    for Vec<T>
{
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Vec<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_seq(DeserializeSerdeContext::<Vec<T>>::new(ctx))
    }
}

impl<'de, T: DeserializeWithSerdeContextDyn> Visitor<'de>
    for DeserializeSerdeContext<'_, '_, Vec<T>>
{
    type Value = Vec<T>;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str("a sequence of values for Vec<T>")
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Vec<T>, A::Error> {
        let seed = ErasedSeed::<T>::new(self.ctx);
        let mut list =
            Vec::with_capacity(seq.size_hint().unwrap_or(0).min(4096));
        while let Some(value) = seq.next_element_seed(seed)? {
            list.push(value);
        }
        Ok(list)
    }
}
