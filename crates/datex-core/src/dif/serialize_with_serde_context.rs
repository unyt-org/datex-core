use crate::{
    dif::{
        serde_context::SerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
};
use serde::Serializer;

/// A trait for types that can be serialized with a `SerdeContext`.
pub trait SerializeWithSerdeContext {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>;
}

impl<T: SerializeWithSerdeContext + ?Sized> SerializeWithSerdeContext for &T {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        (**self).serialize_with_ctx(ctx, serializer)
    }
}

impl<T: SerializeWithSerdeContext + ?Sized> SerializeWithSerdeContext
    for &mut T
{
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        (**self).serialize_with_ctx(ctx, serializer)
    }
}

/// Object-safe version: returns something serde can serialize.
pub trait SerializeWithSerdeContextDyn {
    fn with_ctx<'a>(
        &'a self,
        ctx: &'a SerdeContext<'a>,
    ) -> Box<dyn erased_serde::Serialize + 'a>;
}

impl<T: SerializeWithSerdeContext> SerializeWithSerdeContextDyn for T {
    fn with_ctx<'a>(
        &'a self,
        ctx: &'a SerdeContext<'a>,
    ) -> Box<dyn erased_serde::Serialize + 'a> {
        Box::new(ValueWithSerdeContext { value: self, ctx })
    }
}
