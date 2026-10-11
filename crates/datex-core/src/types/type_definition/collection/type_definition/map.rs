use crate::{
    dif::serde_context::SerdeContext,
    dif::serialize_with_serde_context::SerializeWithSerdeContext,
};
use core::fmt::{self, Display};
use serde::{
    Deserializer, Serializer,
    de::{self, DeserializeSeed, SeqAccess, Visitor},
    ser::SerializeSeq,
};
use crate::dif::deserialize_serde_context::DeserializeSerdeContext;
use crate::dif::value_with_serde_context::ValueWithSerdeContext;
use crate::types::r#type::Type;

use crate::prelude::*;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct MapCollectionTypeDefinition {
    pub key_type: Box<Type>,
    pub value_type: Box<Type>,
}
impl MapCollectionTypeDefinition {
    pub fn new(key_type: Type, value_type: Type) -> Self {
        Self {
            key_type: Box::new(key_type),
            value_type: Box::new(value_type),
        }
    }
}
impl Display for MapCollectionTypeDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(f, "Map<{}, {}>", self.key_type, self.value_type)
    }
}

impl SerializeWithSerdeContext for MapCollectionTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&ValueWithSerdeContext::new(
            &self.key_type as &Type,
            ctx,
        ))?;
        seq.serialize_element(&ValueWithSerdeContext::new(
            &self.value_type as &Type,
            ctx,
        ))?;
        seq.end()
    }
}

/// Deserialization implementations for [MapCollectionTypeDefinition].
impl<'de> DeserializeWithSerdeContext<'de> for MapCollectionTypeDefinition
{
    fn deserialize_with_ctx<D>(ctx: &SerdeContext<'_>, deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, DeserializeSerdeContext::<MapCollectionTypeDefinition>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, MapCollectionTypeDefinition>
{
    type Value = MapCollectionTypeDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let key_type = seq
            .next_element_seed(self.cast::<Type>())?
            .ok_or_else(|| de::Error::custom("expected a key type"))?;

        let value_type = seq
            .next_element_seed(self.cast::<Type>())?
            .ok_or_else(|| de::Error::custom("expected a value type"))?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::custom("expected exactly 2 elements"));
        }

        Ok(MapCollectionTypeDefinition {
            key_type: Box::new(key_type),
            value_type: Box::new(value_type),
        })
    }
}
