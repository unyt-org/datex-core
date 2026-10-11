use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        pointer_address::PointerAddressWithOwnership,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    prelude::*,
    shared_values::{
        OwnedSharedContainer, SharedContainer, SharedContainerOwnership,
    },
    traits::clone_unsafe::CloneUnsafe,
};
use erased_serde::__private::serde::{Deserializer, Serialize, Serializer};
use serde::Deserialize;

impl<'de> DeserializeWithSerdeContext<'de> for OwnedSharedContainer {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        d: D,
    ) -> Result<OwnedSharedContainer, D::Error> {
        let PointerAddressWithOwnership { address, ownership } =
            PointerAddressWithOwnership::deserialize(d)?;
        match ownership {
            // must be owned
            SharedContainerOwnership::Owned => ctx
                .shared_container_cache
                .borrow_mut()
                .try_take_owned_shared_container(&address)
                .map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Failed to retrieve shared container from cache: {}",
                        e
                    ))
                }),
            _ => Err(serde::de::Error::custom(
                "Expected owned shared container, but got a reference",
            )),
        }
    }
}

impl SerializeWithSerdeContext for OwnedSharedContainer {
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
        let container = SharedContainer::Owned(unsafe { self.clone_unsafe() });
        unsafe { ctx.pointer_string(&container).serialize(serializer) }
    }
}
