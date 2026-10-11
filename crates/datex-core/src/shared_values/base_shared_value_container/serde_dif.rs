use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    shared_values::base_shared_value_container::BaseSharedValueContainer,
    types::type_definition::TypeDefinition,
    values::value_container::ValueContainer,
};
use core::fmt;
use serde::{
    Serializer,
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
};

impl SerializeWithSerdeContext for BaseSharedValueContainer {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // serialize as struct
        let mut state = serializer.serialize_seq(Some(3))?;

        state.serialize_element(&ValueWithSerdeContext::new(
            &self.value_container,
            ctx,
        ))?;

        state.serialize_element(&self.mutability)?;
        state.serialize_element(&ValueWithSerdeContext::new(
            &self.allowed_type,
            ctx,
        ))?;
        state.end()
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for BaseSharedValueContainer {
    fn deserialize_with_ctx<D: serde::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(DeserializeSerdeContext::<
            BaseSharedValueContainer,
        >::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, BaseSharedValueContainer>
{
    type Value = BaseSharedValueContainer;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a valid BaseSharedValueContainer")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let value_container = seq
            .next_element_seed(self.cast::<ValueContainer>())?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

        let mutability = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

        let allowed_type =
            seq.next_element_seed(self.cast::<TypeDefinition>())?;

        Ok(if let Some(allowed_type) = allowed_type {
            BaseSharedValueContainer::try_new(
                value_container,
                allowed_type,
                mutability,
            )
            .map_err(|err| {
                serde::de::Error::custom(format!(
                    "invalid BaseSharedValueContainer: {err}"
                ))
            })?
        } else {
            BaseSharedValueContainer::new_with_inferred_allowed_type(
                value_container,
                mutability,
            )
        })
    }
}
