use serde::{de, Deserializer};
use serde::de::DeserializeSeed;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;
use crate::dif::serde_context::SerdeContext;
use crate::traits::convert_value::ConvertValue;
use crate::values::value::Value;

// Serde erased deserialize function that deserializes into a Value
pub type DeFn = for<'a> fn(
    &SerdeContext<'_>,
    &mut dyn erased_serde::Deserializer<'a>,
) -> Result<Value, erased_serde::Error>;

// helper function to convert [T] that implements [DeserializeWithSerdeContext] to a [DeFn].
pub fn de_as<T>(
    ctx: &SerdeContext<'_>,
    d: &mut dyn erased_serde::Deserializer<'_>,
) -> Result<Value, erased_serde::Error>
where
    T: for<'de> DeserializeWithSerdeContext<'de> + ConvertValue,
{
    T::deserialize_with_ctx(ctx, d).map(|v| v.to_value())
}

pub struct DynSeed<'c, 'x> {
    pub(crate) f: DeFn,
    pub(crate) ctx: &'c SerdeContext<'x>,
}

impl<'de> DeserializeSeed<'de> for DynSeed<'_, '_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        let mut erased = <dyn erased_serde::Deserializer>::erase(d);
        (self.f)(self.ctx, &mut erased).map_err(de::Error::custom)
    }
}