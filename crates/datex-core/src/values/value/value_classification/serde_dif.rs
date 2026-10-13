use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    shared_values::{PointerAddress, SharedContainer},
    types::entity_type::EntityType,
    values::value::value_classification::{ValueClassification, ValueTag},
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
};
use crate::values::value::value_classification::unresolved_value_classification::UnresolvedValueClassification;

/// Serialization for [ValueClassification].
impl SerializeWithSerdeContext for ValueClassification {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        // Serialize entity_type if it exists
        if let Some(entity_type) = &self.entity_type {
            seq.serialize_element(&ValueWithSerdeContext::new(
                entity_type.shared_container(),
                ctx,
            ))?;
            if self.impls.is_empty() && self.tag.is_none() {
                return seq.end();
            }
        } else {
            seq.serialize_element(&Option::<()>::None)?;
        }

        // tag is handled in layer below

        // Serialize impls if they exist
        if !self.impls.is_empty() {
            seq.serialize_element(&self.impls)?;
        }

        seq.end()
    }
}

impl Serialize for UnresolvedValueClassification {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        // Serialize entity_type_address if it exists
        if let Some(entity_type_address) = &self.entity_type_address {
            seq.serialize_element(&entity_type_address)?;
            if self.impls.is_empty() && self.tag.is_none() {
                return seq.end();
            }
        } else {
            seq.serialize_element(&Option::<()>::None)?;
        }

        // tag is handled in layer below

        // Serialize impls if they exist
        if !self.impls.is_empty() {
            seq.serialize_element(&self.impls)?;
        }

        seq.end()
    }
}


impl<'de> DeserializeWithSerdeContext<'de> for ValueClassification {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeSerdeContext::<
            ValueClassification,
        >::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, ValueClassification>
{
    type Value = ValueClassification;

    fn expecting(
        &self,
        formatter: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        formatter.write_str("a value classification")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // [entity_type?, [tag: string, is_empty?: true]?, impls[]?]
        let entity_type = seq
            .next_element_seed(self.cast::<Option<SharedContainer>>())?
            .flatten()
            .map(|shared| unsafe { EntityType::new_unchecked(shared) });
        let impls: Vec<PointerAddress> = seq
            .next_element::<Option<Vec<PointerAddress>>>()?
            .flatten()
            .unwrap_or_default();

        Ok(ValueClassification {
            entity_type,
            tag: None, // tag is handled in layer below
            impls,
        })
    }
}

impl Serialize for ValueTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(if self.is_empty {
            Some(2)
        } else {
            Some(1)
        })?;
        seq.serialize_element(&self.tag)?;
        if self.is_empty {
            seq.serialize_element(&true)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for ValueTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ValueTagVisitor)
    }
}

struct ValueTagVisitor;

impl<'de, 'a, 'ctx> Visitor<'de> for ValueTagVisitor {
    type Value = ValueTag;

    fn expecting(
        &self,
        formatter: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        formatter.write_str("a value tag")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let tag: String = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("Expected a tag string"))?;
        let is_empty: Option<bool> = seq.next_element()?;

        Ok(ValueTag {
            tag,
            is_empty: is_empty.unwrap_or(false),
        })
    }
}
