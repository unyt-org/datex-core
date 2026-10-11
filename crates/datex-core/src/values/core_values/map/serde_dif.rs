use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    values::{
        core_values::map::{Map, MapEntries},
        value_container::ValueContainer,
    },
};
use core::fmt;
use indexmap::IndexMap;
use serde::{
    Deserializer, Serializer,
    de::{MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq, SerializeTuple},
};

impl SerializeWithSerdeContext for (ValueContainer, ValueContainer) {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;

        tuple.serialize_element(&ValueWithSerdeContext::new(&self.0, ctx))?;

        tuple.serialize_element(&ValueWithSerdeContext::new(&self.1, ctx))?;

        tuple.end()
    }
}

impl SerializeWithSerdeContext for Map {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.entries {
            MapEntries::StructuralWithStringKeys(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;

                for (key, value) in entries {
                    map.serialize_key(key)?;
                    map.serialize_value(&ValueWithSerdeContext::new(
                        value, ctx,
                    ))?;
                }

                map.end()
            }

            MapEntries::Structural(entries) => {
                entries.serialize_with_ctx(ctx, serializer)
            }

            MapEntries::Dynamic(entries) => {
                let mut seq = serializer.serialize_seq(Some(entries.len()))?;

                for (key, value) in entries {
                    let entry = (key.clone(), value.clone());

                    seq.serialize_element(&ValueWithSerdeContext::new(
                        &entry, ctx,
                    ))?;
                }

                seq.end()
            }
        }
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Map {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DeserializeSerdeContext::<Map>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, Map> {
    type Value = Map;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(
            "either an object with string keys or a sequence of [key, value] entries",
        )
    }

    fn visit_map<A>(self, mut map: A) -> Result<Map, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Vec::new();

        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(self.cast::<ValueContainer>())?;

            entries.push((key, value));
        }

        Ok(MapEntries::StructuralWithStringKeys(entries).into())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Map, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut entries = Vec::new();

        while let Some(entry) = seq.next_element_seed(
            self.cast::<(ValueContainer, ValueContainer)>(),
        )? {
            entries.push(entry);
        }

        Ok(MapEntries::Dynamic(IndexMap::from_iter(entries)).into())
    }
}
