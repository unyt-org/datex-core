use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    shared_values::SharedContainer,
    types::entity_type::EntityType,
};
use serde::{Deserializer, Serialize, Serializer, de::DeserializeSeed};

impl<'de> DeserializeWithSerdeContext<'de> for EntityType {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        Ok(unsafe {
            EntityType::new_unchecked(
                DeserializeSerdeContext::<SharedContainer>::new(ctx)
                    .deserialize(deserializer)?,
            )
        })
    }
}
impl SerializeWithSerdeContext for EntityType {
    /// SAFETY:
    /// The caller of the `serialize` method must either
    /// * guarantee that no direct value (accessible without borrow) is an owned shared value
    ///   (this can be guaranteed by calling clone on the top level value before passing it to [SerializeWithSerdeContext])
    /// * or guarantee that the value is dropped after calling `serialize`, so that the owned shared value
    ///   is not leaked after serialization.
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize_with_ctx(ctx, serializer)
    }
}
