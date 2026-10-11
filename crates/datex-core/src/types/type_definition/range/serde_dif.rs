use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    types::{r#type::Type, type_definition::range::RangeTypeDefinition},
};
use serde::{
    Deserializer, Serializer,
    de::{self, SeqAccess, Visitor},
    ser::SerializeTuple,
};

impl SerializeWithSerdeContext for RangeTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&ValueWithSerdeContext::new(
            &*self.start,
            ctx,
        ))?;
        tuple
            .serialize_element(&ValueWithSerdeContext::new(&*self.end, ctx))?;
        tuple.end()
    }
}

/// Deserialization implementations for [RangeTypeDefinition].
impl<'de> DeserializeWithSerdeContext<'de> for RangeTypeDefinition {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeSerdeContext::<
            RangeTypeDefinition,
        >::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, RangeTypeDefinition>
{
    type Value = RangeTypeDefinition;

    fn expecting(
        &self,
        formatter: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        formatter
            .write_str("a tuple of two Type definitions representing a range")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let start = seq
            .next_element_seed(self.cast::<Type>())?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        let end = seq
            .next_element_seed(self.cast::<Type>())?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        Ok(RangeTypeDefinition {
            start: Box::new(start),
            end: Box::new(end),
        })
    }
}
