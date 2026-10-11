use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        pointer_address::PointerAddressWithOwnership,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    prelude::*,
    shared_values::{
        ReferencedSharedContainer, SharedContainer, SharedContainerOwnership,
    },
};
use erased_serde::__private::serde::{Deserializer, Serialize, Serializer};
use serde::Deserialize;

impl<'de> DeserializeWithSerdeContext<'de> for ReferencedSharedContainer {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        d: D,
    ) -> Result<ReferencedSharedContainer, D::Error> {
        let PointerAddressWithOwnership { address, ownership } =
            PointerAddressWithOwnership::deserialize(d)?;
        match ownership {
            // must be owned
            SharedContainerOwnership::Referenced(mutability) => ctx
                .shared_container_cache
                .borrow_mut()
                .try_get_shared_container_reference(&address, mutability)
                .map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Failed to retrieve shared container from cache: {}",
                        e
                    ))
                }),
            _ => Err(serde::de::Error::custom(
                "Expected referenced shared container, but got an owned container",
            )),
        }
    }
}

impl SerializeWithSerdeContext for ReferencedSharedContainer {
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
        let container = SharedContainer::Referenced(self.clone());
        unsafe { ctx.pointer_string(&container).serialize(serializer) }
    }
}
