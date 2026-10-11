use crate::{
    dif::{
        deserialize_with_serde_context::{
            DeserializeWithSerdeContext, DeserializeWithSerdeContextDyn,
        },
        serde_context::SerdeContext,
        serialize_with_serde_context::{
            SerializeWithSerdeContext, SerializeWithSerdeContextDyn,
        },
    },
    prelude::*,
};
use serde::{Serialize, Serializer};

impl<T: SerializeWithSerdeContextDyn> SerializeWithSerdeContext for Box<T> {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        (*self).with_ctx(ctx).serialize(serializer)
    }
}

impl<'de, T: DeserializeWithSerdeContextDyn> DeserializeWithSerdeContext<'de>
    for Box<T>
{
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        _ctx: &SerdeContext<'_>,
        _deserializer: D,
    ) -> Result<Self, D::Error> {
        todo!()
    }
}
