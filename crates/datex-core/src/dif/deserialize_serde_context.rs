use crate::dif::{
    deserialize_with_serde_context::{
        DeserializeWithSerdeContext, DeserializeWithSerdeContextDyn,
    },
    serde_context::SerdeContext,
};
use core::marker::PhantomData;
use serde::{
    Deserialize, Deserializer,
    de::{DeserializeSeed, Error},
};

pub struct DeserializeSerdeContext<'a, 'ctx, T> {
    pub ctx: &'a SerdeContext<'ctx>,
    _marker: PhantomData<T>,
}

impl<'a, 'ctx, T> DeserializeSerdeContext<'a, 'ctx, T> {
    pub fn new(ctx: &'a SerdeContext<'ctx>) -> Self {
        Self {
            ctx,
            _marker: PhantomData,
        }
    }

    pub fn cast<U>(&self) -> DeserializeSerdeContext<'a, 'ctx, U> {
        DeserializeSerdeContext {
            ctx: self.ctx,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for DeserializeSerdeContext<'_, '_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for DeserializeSerdeContext<'_, '_, T> {}

impl<'de, T: DeserializeWithSerdeContext<'de>> DeserializeSeed<'de>
    for DeserializeSerdeContext<'_, '_, T>
{
    type Value = T;

    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<T, D::Error> {
        T::deserialize_with_ctx(self.ctx, deserializer)
    }
}

pub struct ErasedSeed<'a, 'ctx, T> {
    ctx: &'a SerdeContext<'ctx>,
    _marker: PhantomData<fn() -> T>,
}
impl<'a, 'ctx, T> ErasedSeed<'a, 'ctx, T> {
    pub fn new(ctx: &'a SerdeContext<'ctx>) -> Self {
        Self {
            ctx,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for ErasedSeed<'_, '_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ErasedSeed<'_, '_, T> {}

impl<'de, T: DeserializeWithSerdeContextDyn> DeserializeSeed<'de>
    for ErasedSeed<'_, '_, T>
{
    type Value = T;

    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<T, D::Error> {
        let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
        T::deserialize_with_ctx_dyn(self.ctx, &mut erased)
            .map_err(D::Error::custom)
    }
}

pub macro impl_serde_with_context($t:ty) {
    impl<'de> DeserializeWithSerdeContext<'de> for $t {
        fn deserialize_with_ctx<D: Deserializer<'de>>(
            _ctx: &SerdeContext<'_>,
            deserializer: D,
        ) -> Result<Self, D::Error> {
            <$t>::deserialize(deserializer)
        }
    }

    impl $crate::dif::serialize_with_serde_context::SerializeWithSerdeContext
        for $t
    {
        fn serialize_with_ctx<S: serde::ser::Serializer>(
            &self,
            _ctx: &SerdeContext<'_>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            serde::Serialize::serialize(self, serializer)
        }
    }
}
