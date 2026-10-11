use crate::dif::serde_context::SerdeContext;
use serde::Deserializer;

/// A trait for types that can be deserialized with a `SerdeContext`.
pub trait DeserializeWithSerdeContext<'de>: Sized {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>;
}

/// Non-generic version: takes an erased deserializer.
pub trait DeserializeWithSerdeContextDyn {
    fn deserialize_with_ctx_dyn<'de>(
        ctx: &SerdeContext<'_>,
        deserializer: &mut dyn erased_serde::Deserializer<'de>,
    ) -> Result<Self, erased_serde::Error>
    where
        Self: Sized;
}

impl<T> DeserializeWithSerdeContextDyn for T
where
    T: for<'de> DeserializeWithSerdeContext<'de>,
{
    fn deserialize_with_ctx_dyn<'de>(
        ctx: &SerdeContext<'_>,
        deserializer: &mut dyn erased_serde::Deserializer<'de>,
    ) -> Result<Self, erased_serde::Error> {
        <T as DeserializeWithSerdeContext<'de>>::deserialize_with_ctx(
            ctx,
            deserializer,
        )
    }
}
