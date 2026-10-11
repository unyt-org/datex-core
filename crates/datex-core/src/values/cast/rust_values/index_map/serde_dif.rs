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
    random::RandomState,
};
use core::{fmt, hash::Hash};
use erased_serde::__private::serde::{de, de::MapAccess};
use indexmap::IndexMap;
use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
};

impl<'ctx, K: SerializeWithSerdeContextDyn, V: SerializeWithSerdeContextDyn>
    SerializeWithSerdeContext for IndexMap<K, V, RandomState>
{
    fn serialize_with_ctx<S: serde::Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for (key, value) in self {
            seq.serialize_element(&(key.with_ctx(ctx), value.with_ctx(ctx)))?;
        }
        seq.end()
    }
}

impl<
    'de,
    'ctx,
    K: DeserializeWithSerdeContextDyn + Eq + Hash,
    V: DeserializeWithSerdeContextDyn,
> DeserializeWithSerdeContext<'de> for IndexMap<K, V, RandomState>
where
    (K, V): DeserializeWithSerdeContextDyn,
{
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<IndexMap<K, V, RandomState>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeSerdeContext::<
            IndexMap<K, V, RandomState>,
        >::new(ctx))
    }
}

impl<
    'de,
    'a,
    'ctx,
    K: DeserializeWithSerdeContextDyn + Eq + Hash,
    V: DeserializeWithSerdeContextDyn,
> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, IndexMap<K, V, RandomState>>
where
    (K, V): DeserializeWithSerdeContextDyn,
{
    type Value = IndexMap<K, V, RandomState>;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str("a sequence of values for IndexMap<K, V, RandomState>")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let seed = ErasedSeed::<(K, V)>::new(self.ctx);
        let mut index_map = IndexMap::default();

        while let Some((key, value)) = seq.next_element_seed(seed)? {
            index_map.insert(key, value);
        }

        Ok(index_map)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let key_seed = ErasedSeed::<K>::new(self.ctx);
        let value_seed = ErasedSeed::<V>::new(self.ctx);

        let mut index_map = IndexMap::default();

        while let Some(key) = map.next_key_seed(key_seed)? {
            let value = map.next_value_seed(value_seed)?;

            index_map.insert(key, value);
        }

        Ok(index_map)
    }
}

impl<'de, A, B> DeserializeWithSerdeContext<'de> for (A, B)
where
    A: DeserializeWithSerdeContextDyn,
    B: DeserializeWithSerdeContextDyn,
{
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_tuple(2, DeserializeSerdeContext::<(A, B)>::new(ctx))
    }
}

/// Visitor implementations for deserialization of inner tuple type `(A, B)`.
impl<'de, 'a, 'ctx, A, B> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, (A, B)>
where
    A: DeserializeWithSerdeContextDyn,
    B: DeserializeWithSerdeContextDyn,
{
    type Value = (A, B);

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a type-definition key/value tuple")
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let seed_a = ErasedSeed::<A>::new(self.ctx);
        let seed_b = ErasedSeed::<B>::new(self.ctx);

        let key = seq
            .next_element_seed(seed_a)?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        let value = seq
            .next_element_seed(seed_b)?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::invalid_length(3, &self));
        }

        Ok((key, value))
    }
}
