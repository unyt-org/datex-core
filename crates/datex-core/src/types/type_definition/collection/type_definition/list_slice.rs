use core::fmt::Display;

use crate::dif::{
    serde_context::SerdeContext,
    serialize_with_serde_context::SerializeWithSerdeContext,
};
use serde::{Serializer, ser::SerializeSeq};

use crate::types::r#type::Type;

use crate::prelude::*;
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct ListSliceCollectionTypeDefinition {
    pub item_type: Box<Type>,
    pub size: usize,
}
impl ListSliceCollectionTypeDefinition {
    pub fn new(item_type: Type, size: usize) -> Self {
        Self {
            item_type: Box::new(item_type),
            size,
        }
    }
}
impl Display for ListSliceCollectionTypeDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(f, "[{}; {}]", self.item_type, self.size)
    }
}

impl SerializeWithSerdeContext for ListSliceCollectionTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&ValueWithSerdeContext::new(
            &self.item_type as &Type,
            ctx,
        ))?;
        seq.serialize_element(&self.size)?;
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
    de::{self, SeqAccess, Visitor},
};

impl<'de> DeserializeWithSerdeContext<'de>
    for ListSliceCollectionTypeDefinition
{
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(
            2,
            DeserializeSerdeContext::<ListSliceCollectionTypeDefinition>::new(
                ctx,
            ),
        )
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, ListSliceCollectionTypeDefinition>
{
    type Value = ListSliceCollectionTypeDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple [item_type, size]")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let item_type = seq
            .next_element_seed(self.cast::<Type>())?
            .ok_or_else(|| de::Error::custom("expected item type"))?;

        let size = seq
            .next_element()?
            .ok_or_else(|| de::Error::custom("expected size"))?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::custom("expected exactly 2 elements"));
        }

        Ok(ListSliceCollectionTypeDefinition {
            item_type: Box::new(item_type),
            size,
        })
    }
}
