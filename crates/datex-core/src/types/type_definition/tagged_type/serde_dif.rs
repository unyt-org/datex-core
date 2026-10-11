use core::fmt;

use serde::{
    Deserializer, Serializer,
    de::{self, DeserializeSeed, SeqAccess, Visitor},
    ser::SerializeSeq,
};

use crate::{
    dif::{
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    types::type_definition::tagged_type::TaggedTypeDefinition,
};

impl SerializeWithSerdeContext for TaggedTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.tag)?;
        match &self.ty {
            Some(ty) => {
                seq.serialize_element(&ValueWithSerdeContext::new(
                    ty.as_ref(),
                    ctx,
                ))?;
            }
            None => seq.serialize_element(&ValueWithSerdeContext::new(
                &TypeDefinition::CoreType(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Unit,
                )),
                ctx,
            ))?,
        }
        seq.end()
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for TaggedTypeDefinition {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(
            2,
            DeserializeSerdeContext::<TaggedTypeDefinition>::new(ctx),
        )
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, TaggedTypeDefinition>
{
    type Value = TaggedTypeDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple [tag, ty]")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let tag = seq
            .next_element()?
            .ok_or_else(|| de::Error::custom("expected tag"))?;

        let ty = seq
            .next_element_seed(self.cast::<Option<Box<Type>>>())?
            .ok_or_else(|| de::Error::custom("expected ty"))?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::custom("expected exactly 2 elements"));
        }

        Ok(TaggedTypeDefinition { tag, ty })
    }
}

use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId},
    prelude::*,
    types::{r#type::Type, type_definition::TypeDefinition},
};
