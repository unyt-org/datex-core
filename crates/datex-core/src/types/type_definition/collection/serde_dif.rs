use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    types::type_definition::{
        collection::{
            CollectionTypeDefinition,
            type_definition::{
                list::ListCollectionTypeDefinition,
                list_slice::ListSliceCollectionTypeDefinition,
                map::MapCollectionTypeDefinition,
            },
        },
        range::RangeTypeDefinition,
    },
};
use serde::{
    Deserializer, Serializer,
    de::{self, Visitor},
    ser::{SerializeMap, SerializeSeq},
};

impl SerializeWithSerdeContext for CollectionTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut obj = serializer.serialize_map(Some(1))?;
        obj.serialize_key(self.as_ref())?;
        match self {
            CollectionTypeDefinition::List(item) => {
                obj.serialize_value(&ValueWithSerdeContext::new(item, ctx))?
            }
            CollectionTypeDefinition::ListSlice(item) => {
                obj.serialize_value(&ValueWithSerdeContext::new(item, ctx))?
            }
            CollectionTypeDefinition::Map(map) => {
                obj.serialize_value(&ValueWithSerdeContext::new(map, ctx))?
            }
            CollectionTypeDefinition::Range(range) => {
                obj.serialize_value(&ValueWithSerdeContext::new(range, ctx))?
            }
        }
        obj.end()
    }
}

/// Deserialization implementations for [CollectionTypeDefinition].
impl<'de> DeserializeWithSerdeContext<'de> for CollectionTypeDefinition {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DeserializeSerdeContext::<
            CollectionTypeDefinition,
        >::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, CollectionTypeDefinition>
{
    type Value = CollectionTypeDefinition;

    fn expecting(
        &self,
        formatter: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        formatter.write_str(
            "a map with a single key representing the collection type",
        )
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let key = map.next_key::<String>()?.ok_or_else(|| {
            de::Error::custom(
                "expected a single key for collection type definition",
            )
        })?;
        match key.as_str() {
            "List" => Ok(CollectionTypeDefinition::List(map.next_value_seed(
                self.cast::<ListCollectionTypeDefinition>(),
            )?)),
            "ListSlice" => {
                Ok(CollectionTypeDefinition::ListSlice(map.next_value_seed(
                    self.cast::<ListSliceCollectionTypeDefinition>(),
                )?))
            }
            "Map" => Ok(CollectionTypeDefinition::Map(map.next_value_seed(
                self.cast::<MapCollectionTypeDefinition>(),
            )?)),
            "Range" => Ok(CollectionTypeDefinition::Range(
                map.next_value_seed(self.cast::<RangeTypeDefinition>())?,
            )),

            _ => Err(de::Error::unknown_variant(
                &key,
                &["List", "ListSlice", "Map", "Range"],
            )),
        }
    }
}
