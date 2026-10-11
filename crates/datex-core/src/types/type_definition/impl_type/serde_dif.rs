use serde::{Serializer, de::DeserializeSeed, ser::SerializeSeq};

use crate::{
    dif::serde_context::SerdeContext,
    types::{r#type::Type, type_definition::impl_type::ImplMarkers},
    dif::serialize_with_serde_context::SerializeWithSerdeContext,
};

use crate::prelude::*;
impl SerializeWithSerdeContext for ImplMarkers {

    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.impl_markers)?;
        seq.end()
    }
}

use core::fmt;
use serde::{
    Deserializer,
    de::{self, SeqAccess, Visitor},
};
use crate::dif::deserialize_serde_context::DeserializeSerdeContext;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;

impl<'de> DeserializeWithSerdeContext<'de> for ImplMarkers
{
    fn deserialize_with_ctx<D>(ctx: &SerdeContext<'_>, deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, DeserializeSerdeContext::<ImplMarkers>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, ImplMarkers> {
    type Value = ImplMarkers;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple [inner_type, impl_markers]")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let impl_markers = seq
            .next_element()?
            .ok_or_else(|| de::Error::custom("expected impl markers"))?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::custom("expected exactly 2 elements"));
        }

        Ok(ImplMarkers {
            impl_markers,
        })
    }
}
