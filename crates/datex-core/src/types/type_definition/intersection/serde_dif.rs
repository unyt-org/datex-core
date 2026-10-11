use serde::{Serializer, de::DeserializeSeed, ser::SerializeSeq};

use crate::{
    dif::serde_context::SerdeContext,
    types::{
        r#type::Type, type_definition::intersection::IntersectionTypeDefinition,
    },
    dif::serialize_with_serde_context::SerializeWithSerdeContext,
};

impl SerializeWithSerdeContext for IntersectionTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for type_def in self.iter() {
            seq.serialize_element(&ValueWithSerdeContext::new(
                type_def,
                ctx,
            ))?;
        }
        seq.end()
    }
}

use core::fmt;
use serde::{
    Deserializer,
    de::{SeqAccess, Visitor},
};
use crate::dif::deserialize_serde_context::DeserializeSerdeContext;
use crate::dif::value_with_serde_context::ValueWithSerdeContext;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;

impl<'de> DeserializeWithSerdeContext<'de> for IntersectionTypeDefinition {

    fn deserialize_with_ctx<D>(ctx: &SerdeContext<'_>, deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeSerdeContext::<IntersectionTypeDefinition>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, IntersectionTypeDefinition>
{
    type Value = IntersectionTypeDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .write_str("a sequence of types for IntersectionTypeDefinition")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut result = IntersectionTypeDefinition::default();

        while let Some(type_def) = seq.next_element_seed(self.cast::<Type>())? {
            result.0.push(type_def);
        }

        Ok(result)
    }
}
