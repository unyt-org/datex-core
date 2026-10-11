use serde::{Serializer, ser::SerializeSeq};

use crate::{
    dif::{
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    types::{r#type::Type, type_definition::union::UnionTypeDefinition},
};

impl SerializeWithSerdeContext for UnionTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for type_def in self.iter() {
            seq.serialize_element(&ValueWithSerdeContext::new(type_def, ctx))?;
        }
        seq.end()
    }
}

use crate::dif::{
    deserialize_serde_context::DeserializeSerdeContext,
    deserialize_with_serde_context::DeserializeWithSerdeContext,
    value_with_serde_context::ValueWithSerdeContext,
};
use core::fmt;
use serde::{
    Deserializer,
    de::{SeqAccess, Visitor},
};

impl<'de> DeserializeWithSerdeContext<'de> for UnionTypeDefinition {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(DeserializeSerdeContext::<
            UnionTypeDefinition,
        >::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, UnionTypeDefinition>
{
    type Value = UnionTypeDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence of types for UnionTypeDefinition")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut result = UnionTypeDefinition::default();

        while let Some(type_def) = seq.next_element_seed(self.cast::<Type>())? {
            result.0.push(type_def);
        }

        Ok(result)
    }
}
