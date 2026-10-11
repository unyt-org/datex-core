use crate::{
    dif::serde_context::SerdeContext,
    dif::serialize_with_serde_context::SerializeWithSerdeContext,
};
use core::fmt::Display;
use serde::{Deserializer, Serializer, de::DeserializeSeed};

use crate::{prelude::*, types::r#type::Type};
use crate::dif::deserialize_serde_context::DeserializeSerdeContext;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct ListCollectionTypeDefinition(pub Box<Type>);
impl Display for ListCollectionTypeDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(f, "[{}]", self.0)
    }
}
impl ListCollectionTypeDefinition {
    pub fn new(item: Type) -> Self {
        Self(Box::new(item))
    }
}
impl SerializeWithSerdeContext for ListCollectionTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_with_ctx(ctx, serializer)
    }
}

/// Deserialization implementations for [ListCollectionTypeDefinition].
impl<'de> DeserializeWithSerdeContext<'de> for ListCollectionTypeDefinition
{
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let item_type = DeserializeSerdeContext::<Type>::new(ctx).deserialize(deserializer)?;
        Ok(ListCollectionTypeDefinition(Box::new(item_type)))
    }
}
