use serde::de::Error;
use serde::{Deserialize, Serializer};
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;
use crate::dif::serde_context::SerdeContext;
use crate::dif::serialize_with_serde_context::SerializeWithSerdeContext;
use crate::libs::core::type_id::CoreLibVariantTypeId;
use crate::values::core_value::CoreValue;
use crate::values::core_values::integer::typed_integer::{IntegerTypeVariant, TypedInteger};
use crate::prelude::*;

macro impl_serde_with_serde_context_for_int($ty:ty, $convert_ty:ty, $variant:ident) {
    impl SerializeWithSerdeContext for $ty {
        fn serialize_with_ctx<S: Serializer>(
            &self,
            ctx: &SerdeContext<'_>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            ctx.serialize_core_value(
                &(*self as $convert_ty),
                CoreLibVariantTypeId::Integer(IntegerTypeVariant::$variant).into(),
                serializer,
                false,
            )
        }
    }

    impl<'de> DeserializeWithSerdeContext<'de> for $ty {
        fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
            ctx: &SerdeContext<'_>,
            deserializer: D,
        ) -> Result<Self, D::Error> {
            match ctx.deserialize_core_value(deserializer)? {
                CoreValue::TypedInteger(TypedInteger::$variant(val)) => Ok(val as $ty),
                _ => Err(D::Error::custom(format!(
                    "Expected CoreValue::TypedInteger({})",
                    stringify!($variant)
                ))),
            }
        }
    }
}

impl_serde_with_serde_context_for_int!(u8, u8, U8);
impl_serde_with_serde_context_for_int!(u16, u16, U16);
impl_serde_with_serde_context_for_int!(u32, u32, U32);
impl_serde_with_serde_context_for_int!(u64, u64, U64);
impl_serde_with_serde_context_for_int!(u128, u128, U128);
impl_serde_with_serde_context_for_int!(i8, i8, I8);
impl_serde_with_serde_context_for_int!(i16, i16, I16);
impl_serde_with_serde_context_for_int!(i32, i32, I32);
impl_serde_with_serde_context_for_int!(i64, i64, I64);
impl_serde_with_serde_context_for_int!(i128, i128, I128);


cfg_select! {
    target_pointer_width = "32" => {
        impl_serde_with_serde_context_for_int!(usize, u32, U32);
        impl_serde_with_serde_context_for_int!(isize, i32, I32);
    },
    target_pointer_width = "64" => {
        impl_serde_with_serde_context_for_int!(usize, u64, U64);
        impl_serde_with_serde_context_for_int!(isize, i64, I64);
    },
}

