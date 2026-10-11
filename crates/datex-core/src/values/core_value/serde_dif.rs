use crate::{
    libs::core::type_id::{
        CoreLibBaseTypeId, CoreLibTypeId, CoreLibVariantTypeId,
    },
    prelude::*,
    values::{
        core_value::CoreValue,
        core_values::{
            decimal::{
                Decimal,
                typed_decimal::{DecimalTypeVariant, TypedDecimal},
            },
            instant::Instant,
            integer::{
                Integer,
                typed_integer::{IntegerTypeVariant, TypedInteger},
            },
            list::List,
            map::Map,
            range::Range,
        },
    },
};
use num::ToPrimitive;
use serde::de::{Error, MapAccess};

use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::core_lib_id::CoreLibIdIndex,
    values::core_values::{
        boolean::Boolean, callable::Callable, endpoint::Endpoint,
    },
};
use core::{fmt, str::FromStr};
use serde::{
    Deserializer, Serializer,
    de::{DeserializeSeed, SeqAccess, Visitor},
};
use serde_with::__private__::DeError;

impl SerializeWithSerdeContext for CoreValue {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let core_lib_type = self.core_lib_type_id();

        match &self {
            // Direct serializable core values, that can be serialized as they can be unambiguously deserialized without it
            CoreValue::Boolean(b) => {
                ctx.serialize_core_value(b, core_lib_type, serializer, true)
            }
            CoreValue::Text(s) => {
                ctx.serialize_core_value(s, core_lib_type, serializer, true)
            }
            CoreValue::Null => {
                ctx.serialize_core_value(&(), core_lib_type, serializer, true)
            }
            CoreValue::TypedDecimal(dec @ TypedDecimal::F64(_)) => ctx
                .serialize_core_value(
                    &dec,
                    core_lib_type,
                    serializer,
                    dec.is_finite(),
                ),

            // Core values that require a specific core type id to be serialized for non-ambiguous deserialization
            CoreValue::Endpoint(endpoint) => ctx.serialize_core_value(
                endpoint,
                core_lib_type,
                serializer,
                false,
            ),
            CoreValue::Decimal(d) => {
                ctx.serialize_core_value(d, core_lib_type, serializer, false)
            }
            CoreValue::Integer(i) => {
                ctx.serialize_core_value(i, core_lib_type, serializer, false)
            }
            CoreValue::TypedInteger(ti) => {
                ctx.serialize_core_value(ti, core_lib_type, serializer, false)
            }
            CoreValue::Instant(instant) => ctx.serialize_core_value(
                &instant,
                core_lib_type,
                serializer,
                false,
            ),
            CoreValue::TypedDecimal(td) => {
                ctx.serialize_core_value(td, core_lib_type, serializer, false)
            }

            // Complex core values, that can contain nested values
            CoreValue::List(l) => ctx.serialize_value_with_context(
                l,
                core_lib_type,
                serializer,
                false,
            ),
            CoreValue::Range(range) => ctx.serialize_value_with_context(
                range,
                core_lib_type,
                serializer,
                false,
            ),

            CoreValue::Map(map) => ctx.serialize_value_with_context(
                map,
                core_lib_type,
                serializer,
                false,
            ),
            CoreValue::Type(ty) => ctx.serialize_value_with_context(
                ty,
                core_lib_type,
                serializer,
                false,
            ),
            CoreValue::EntityTypeDefinition(_entity_type_definition) => {
                todo!()
            }
            CoreValue::Callable(callable) => ctx.serialize_value_with_context(
                callable,
                core_lib_type,
                serializer,
                false,
            ),
            CoreValue::Box(inner) => inner.serialize_with_ctx(ctx, serializer),
            CoreValue::Uninitialized => panic!("Uninitialized value"),
        }
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for CoreValue {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(DeserializeSerdeContext::<CoreValue>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, CoreValue>
{
    type Value = CoreValue;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a value, which can be a direct core value (e.g. boolean, text) or a complex value with a custom type definition")
    }

    /// default mapping for unit: null
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(CoreValue::Null)
    }

    /// default mapping for none: null
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_unit()
    }

    /// default mapping for bool: boolean
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(CoreValue::Boolean(Boolean::new(v)))
    }

    /// default mapping for string: text
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(CoreValue::Text(v.into()))
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_str(&v)
    }

    /// default mapping for f64: decimal/f64
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(CoreValue::TypedDecimal(TypedDecimal::F64(v.into())))
    }

    // default mapping for integers: decimal/f64 (with a check for overflow)
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v as f64)
    }
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v as f64)
    }
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v as f64)
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            DeError::custom(format!(
                "i64 value {v} is too large to fit into f64"
            ))
        })?)
    }
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v as f64)
    }
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v as f64)
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v as f64)
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            DeError::custom(format!(
                "u64 value {v} is too large to fit into f64"
            ))
        })?)
    }
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v as f64)
    }
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            DeError::custom(format!(
                "i128 value {v} is too large to fit into f64"
            ))
        })?)
    }
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            DeError::custom(format!(
                "u128 value {v} is too large to fit into f64"
            ))
        })?)
    }

    /// mapping for [core_lib_type_id, core_value]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let core_lib_type_id: u16 = seq.next_element()?.ok_or_else(|| {
            serde::de::Error::custom("expected a sequence with at least one element for core value deserialization")
        })?;

        let core_lib_id = CoreLibTypeId::try_from(CoreLibIdIndex(
            core_lib_type_id,
        ))
        .map_err(|_| {
            serde::de::Error::custom("invalid core lib id index".to_string())
        })?;

        let visitor = CoreValueVisitor {
            core_lib_id,
            context: &self,
        };
        seq.next_element_seed(visitor)?.ok_or_else(|| {
            serde::de::Error::custom(format!(
                "expected a sequence with at least two elements for core value deserialization, got only one (core lib id: {core_lib_id})"
            ))
        })
    }
}

pub struct CoreValueVisitor<'a, 'ctx> {
    pub core_lib_id: CoreLibTypeId,
    pub context: &'a DeserializeSerdeContext<'a, 'ctx, CoreValue>,
}

impl<'de, 'a, 'ctx> DeserializeSeed<'de> for CoreValueVisitor<'a, 'ctx> {
    type Value = CoreValue;

    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<CoreValue, D::Error> {
        deserializer.deserialize_any(self)
    }
}
impl<'de, 'a, 'ctx> Visitor<'de> for CoreValueVisitor<'a, 'ctx> {
    type Value = CoreValue;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a CoreValue")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match self.core_lib_id {
            CoreLibTypeId::Base(CoreLibBaseTypeId::Boolean) => {
                Ok(CoreValue::Boolean(v.into()))
            }
            other => Err(E::custom(format!(
                "expected CoreValue of type boolean, got {other}"
            ))),
        }
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_i128(v as i128)
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_i128(v as i128)
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_i128(v as i128)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_i128(v as i128)
    }

    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match self.core_lib_id {
            // integer
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I8,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I8(v as i8))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I16,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I16(v as i16))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I32,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I32(v as i32))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I64,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I64(v as i64))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I128,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I128(v))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::IBig,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::IBig(v.into()))),

            // unsigned integer
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U8,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U8(
                v.to_u8().ok_or_else(|| {
                    E::custom(format!("failed to convert integer to u8: {v}"))
                })?,
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U16,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U16(
                v.to_u16().ok_or_else(|| {
                    E::custom(format!("failed to convert integer to u16: {v}"))
                })?,
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U32,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U32(
                v.to_u32().ok_or_else(|| {
                    E::custom(format!("failed to convert integer to u32: {v}"))
                })?,
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U64,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U64(
                v.to_u64().ok_or_else(|| {
                    E::custom(format!("failed to convert integer to u64: {v}"))
                })?,
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U128,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U128(
                v.to_u128().ok_or_else(|| {
                    E::custom(format!("failed to convert integer to u128: {v}"))
                })?,
            ))),

            // base
            CoreLibTypeId::Base(CoreLibBaseTypeId::Integer) => {
                Ok(CoreValue::Integer(v.into()))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal) => {
                Ok(CoreValue::Decimal(
                    Decimal::try_from_string(&v.to_string()).map_err(|e| {
                        E::custom(format!("failed to parse decimal: {e}"))
                    })?,
                ))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Instant) => {
                Ok(CoreValue::Instant(Instant::from_millis(
                    v.to_i128().ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to i128 for Instant: {v}"
                        ))
                    })?,
                )))
            }

            // decimal
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F32,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F32(
                v.to_f32()
                    .ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to f32: {v}"
                        ))
                    })?
                    .into(),
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F64,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F64(
                v.to_f64()
                    .ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to f64: {v}"
                        ))
                    })?
                    .into(),
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::DBig,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::Decimal(
                v.to_f64()
                    .ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to f64 for decimal: {v}"
                        ))
                    })?
                    .into(),
            ))),

            other => Err(E::custom(format!(
                "expected CoreValue of type Integer, got {other}"
            ))),
        }
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_u128(v as u128)
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_u128(v as u128)
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_u128(v as u128)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_u128(v as u128)
    }

    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match self.core_lib_id {
            // integer
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I8,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I8(v as i8))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I16,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I16(v as i16))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I32,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I32(v as i32))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I64,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I64(v as i64))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::I128,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::I128(
                v.to_i128().ok_or_else(|| {
                    E::custom(format!(
                        "failed to convert unsigned integer to i128: {v}"
                    ))
                })?,
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::IBig,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::IBig(v.into()))),

            // unsigned integer
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U8,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U8(v as u8))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U16,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U16(v as u16))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U32,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U32(v as u32))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U64,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U64(v as u64))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                IntegerTypeVariant::U128,
            )) => Ok(CoreValue::TypedInteger(TypedInteger::U128(v))),

            // base
            CoreLibTypeId::Base(CoreLibBaseTypeId::Integer) => {
                Ok(CoreValue::Integer(v.into()))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal) => {
                Ok(CoreValue::Decimal(
                    Decimal::try_from_string(&v.to_string()).map_err(|e| {
                        E::custom(format!("failed to parse decimal: {e}"))
                    })?,
                ))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Instant) => {
                Ok(CoreValue::Instant(Instant::from_millis(
                    v.to_i128().ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to i128 for Instant: {v}"
                        ))
                    })?,
                )))
            }

            // decimal
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F32,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F32(
                v.to_f32()
                    .ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to f32: {v}"
                        ))
                    })?
                    .into(),
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F64,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F64(
                v.to_f64()
                    .ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to f64: {v}"
                        ))
                    })?
                    .into(),
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::DBig,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::Decimal(
                v.to_f64()
                    .ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert integer to f64 for decimal: {v}"
                        ))
                    })?
                    .into(),
            ))),

            other => Err(E::custom(format!(
                "expected CoreValue of type Integer, got {other}"
            ))),
        }
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match self.core_lib_id {
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F32,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F32(v.into()))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F64,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F64(
                (v as f64).into(),
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::DBig,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::Decimal(
                (v as f64).into(),
            ))),
            CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal) => {
                Ok(CoreValue::Decimal(Decimal::from(v)))
            }
            other => Err(E::custom(format!(
                "expected CoreValue of type Decimal, got {other}"
            ))),
        }
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match self.core_lib_id {
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F64,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F64(v.into()))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::F32,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::F32(
                v.to_f32()
                    .ok_or_else(|| {
                        E::custom(format!(
                            "failed to convert decimal to f32: {v}"
                        ))
                    })?
                    .into(),
            ))),
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(
                DecimalTypeVariant::DBig,
            )) => Ok(CoreValue::TypedDecimal(TypedDecimal::Decimal(v.into()))),
            CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal) => {
                Ok(CoreValue::Decimal(Decimal::from(v)))
            }
            other => Err(E::custom(format!(
                "expected CoreValue of type Decimal, got {other}"
            ))),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match self.core_lib_id {
            CoreLibTypeId::Base(CoreLibBaseTypeId::Text) => {
                Ok(CoreValue::Text(v.into()))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Endpoint) => {
                Ok(CoreValue::Endpoint(Endpoint::from_str(v).map_err(|e| {
                    E::custom(format!("failed to parse endpoint: {e}"))
                })?))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Integer) => {
                Ok(CoreValue::Integer(Integer::try_from_string(v).map_err(
                    |e| E::custom(format!("failed to parse integer: {e}")),
                )?))
            }
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(variant)) => {
                Ok(CoreValue::TypedInteger(
                    TypedInteger::try_from_string_and_variant(v, variant)
                        .map_err(|e| {
                            E::custom(format!("failed to parse integer: {e}"))
                        })?,
                ))
            }

            CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal) => {
                Ok(CoreValue::Decimal(Decimal::try_from_string(v).map_err(
                    |e| E::custom(format!("failed to parse decimal: {e}")),
                )?))
            }
            CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(variant)) => {
                Ok(CoreValue::TypedDecimal(
                    TypedDecimal::try_from_string_and_variant(v, variant)
                        .map_err(|e| {
                            E::custom(format!("failed to parse decimal: {e}"))
                        })?,
                ))
            }
            other => Err(E::custom(format!("unexpected {other}"))),
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_unit()
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match self.core_lib_id {
            CoreLibTypeId::Base(CoreLibBaseTypeId::Null) => Ok(CoreValue::Null),
            other => Err(E::custom(format!(
                "expected CoreValue of type Unit, got {other}"
            ))),
        }
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        match self.core_lib_id {
            CoreLibTypeId::Base(CoreLibBaseTypeId::Range) => {
                let elements = self.context.cast::<Range>().visit_seq(seq)?;
                Ok(CoreValue::Range(elements))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::List) => {
                let elements = self.context.cast::<List>().visit_seq(seq)?;
                Ok(CoreValue::List(elements))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Map) => {
                let elements = self.context.cast::<Map>().visit_seq(seq)?;
                Ok(CoreValue::Map(elements))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Callable) => {
                let callable =
                    self.context.cast::<Callable>().visit_seq(seq)?;
                Ok(CoreValue::Callable(callable))
            }
            _ => Err(A::Error::custom(format!(
                "expected CoreValue of type List, Map, or Range, got {}",
                self.core_lib_id
            ))),
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        match self.core_lib_id {
            CoreLibTypeId::Base(CoreLibBaseTypeId::Range) => {
                let elements = self.context.cast::<Range>().visit_map(map)?;
                Ok(CoreValue::Range(elements))
            }
            CoreLibTypeId::Base(CoreLibBaseTypeId::Map) => {
                let elements = self.context.cast::<Map>().visit_map(map)?;
                Ok(CoreValue::Map(elements))
            }
            _ => Err(A::Error::custom(format!(
                "expected CoreValue of type Map, got {}",
                self.core_lib_id
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    use crate::runtime::cache::shared_values_cache::SharedValuesCache;
    use serde::de::DeserializeSeed;
    use serde_json::json;
    use test_case::test_case;

    fn deserialize_value(
        core_lib_id: CoreLibTypeId,
        value: serde_json::Value,
    ) -> Result<CoreValue, serde_json::Error> {
        let cache = RefCell::new(SharedValuesCache::default());
        let context = SerdeContext::new(&cache);
        let mut deserialize_ctx = DeserializeSerdeContext::new(&context);

        CoreValueVisitor {
            core_lib_id,
            context: &mut deserialize_ctx,
        }
        .deserialize(value)
    }

    fn base(base: CoreLibBaseTypeId) -> CoreLibTypeId {
        CoreLibTypeId::Base(base)
    }

    fn int_variant(variant: IntegerTypeVariant) -> CoreLibTypeId {
        CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(variant))
    }

    fn dec_variant(variant: DecimalTypeVariant) -> CoreLibTypeId {
        CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(variant))
    }

    #[test_case(true)]
    #[test_case(false)]
    fn boolean_value_is_deserialized(v: bool) {
        let actual =
            deserialize_value(base(CoreLibBaseTypeId::Boolean), json!(v))
                .unwrap();

        assert_eq!(actual, CoreValue::Boolean(v.into()));
    }

    #[test]
    fn boolean_rejects_non_boolean_core_type() {
        let err = deserialize_value(base(CoreLibBaseTypeId::Text), json!(true))
            .unwrap_err();

        assert!(
            err.to_string()
                .contains("expected CoreValue of type boolean"),
            "{err}"
        );
    }

    #[test_case(-128_i64, IntegerTypeVariant::I8)]
    #[test_case(-32768_i64, IntegerTypeVariant::I16)]
    #[test_case(-2_147_483_648_i64, IntegerTypeVariant::I32)]
    #[test_case(-9_223_372_036_854_775_808_i64, IntegerTypeVariant::I64)]
    #[test_case(-42_i64, IntegerTypeVariant::I128)]
    #[test_case(-42_i64, IntegerTypeVariant::IBig)]
    fn signed_integer_variants_are_deserialized(
        v: i64,
        variant: IntegerTypeVariant,
    ) {
        let actual = deserialize_value(int_variant(variant), json!(v)).unwrap();

        let expected = match variant {
            IntegerTypeVariant::I8 => {
                CoreValue::TypedInteger(TypedInteger::I8(v as i8))
            }
            IntegerTypeVariant::I16 => {
                CoreValue::TypedInteger(TypedInteger::I16(v as i16))
            }
            IntegerTypeVariant::I32 => {
                CoreValue::TypedInteger(TypedInteger::I32(v as i32))
            }
            IntegerTypeVariant::I64 => {
                CoreValue::TypedInteger(TypedInteger::I64(v as i64))
            }
            IntegerTypeVariant::I128 => {
                CoreValue::TypedInteger(TypedInteger::I128(v as i128))
            }
            IntegerTypeVariant::IBig => {
                CoreValue::TypedInteger(TypedInteger::IBig((v as i128).into()))
            }
            other => {
                panic!("unexpected unsigned variant in signed test: {other:?}")
            }
        };

        assert_eq!(actual, expected);
    }

    #[test_case(0_u64, IntegerTypeVariant::U8)]
    #[test_case(255_u64, IntegerTypeVariant::U8)]
    #[test_case(65_535_u64, IntegerTypeVariant::U16)]
    #[test_case(4_294_967_295_u64, IntegerTypeVariant::U32)]
    #[test_case(18_446_744_073_709_551_615_u64, IntegerTypeVariant::U64)]
    #[test_case(42_u64, IntegerTypeVariant::U128)]
    #[test_case(42_u64, IntegerTypeVariant::I8)]
    #[test_case(42_u64, IntegerTypeVariant::I16)]
    #[test_case(42_u64, IntegerTypeVariant::I32)]
    #[test_case(42_u64, IntegerTypeVariant::I64)]
    #[test_case(42_u64, IntegerTypeVariant::I128)]
    #[test_case(42_u64, IntegerTypeVariant::IBig)]
    fn unsigned_integer_variants_are_deserialized(
        v: u64,
        variant: IntegerTypeVariant,
    ) {
        let actual = deserialize_value(int_variant(variant), json!(v)).unwrap();

        let expected = match variant {
            IntegerTypeVariant::I8 => {
                CoreValue::TypedInteger(TypedInteger::I8(v as i8))
            }
            IntegerTypeVariant::I16 => {
                CoreValue::TypedInteger(TypedInteger::I16(v as i16))
            }
            IntegerTypeVariant::I32 => {
                CoreValue::TypedInteger(TypedInteger::I32(v as i32))
            }
            IntegerTypeVariant::I64 => {
                CoreValue::TypedInteger(TypedInteger::I64(v as i64))
            }
            IntegerTypeVariant::I128 => {
                CoreValue::TypedInteger(TypedInteger::I128(v as i128))
            }
            IntegerTypeVariant::IBig => {
                CoreValue::TypedInteger(TypedInteger::IBig((v as u128).into()))
            }
            IntegerTypeVariant::U8 => {
                CoreValue::TypedInteger(TypedInteger::U8(v as u8))
            }
            IntegerTypeVariant::U16 => {
                CoreValue::TypedInteger(TypedInteger::U16(v as u16))
            }
            IntegerTypeVariant::U32 => {
                CoreValue::TypedInteger(TypedInteger::U32(v as u32))
            }
            IntegerTypeVariant::U64 => {
                CoreValue::TypedInteger(TypedInteger::U64(v as u64))
            }
            IntegerTypeVariant::U128 => {
                CoreValue::TypedInteger(TypedInteger::U128(v as u128))
            }
        };

        assert_eq!(actual, expected);
    }

    #[test_case(json!(-123); "negative integer")]
    #[test_case(json!(0); "zero")]
    #[test_case(json!(123); "positive integer")]
    fn base_integer_accepts_json_numbers(value: serde_json::Value) {
        let expected = value.as_i64().unwrap();

        let actual =
            deserialize_value(base(CoreLibBaseTypeId::Integer), value).unwrap();

        assert_eq!(actual, CoreValue::Integer(expected.into()));
    }

    #[test_case("0")]
    #[test_case("-1")]
    #[test_case("123456789012345678901234567890")]
    fn base_integer_accepts_string_numbers(v: &str) {
        let actual =
            deserialize_value(base(CoreLibBaseTypeId::Integer), json!(v))
                .unwrap();

        assert_eq!(
            actual,
            CoreValue::Integer(Integer::try_from_string(v).unwrap())
        );
    }

    #[test_case("abc")]
    #[test_case("12.34")]
    #[test_case("")]
    fn base_integer_rejects_invalid_string_numbers(v: &str) {
        let err = deserialize_value(base(CoreLibBaseTypeId::Integer), json!(v))
            .unwrap_err();

        assert!(err.to_string().contains("failed to parse integer"), "{err}");
    }

    #[test_case("127", IntegerTypeVariant::I8)]
    #[test_case("-128", IntegerTypeVariant::I8)]
    #[test_case("32767", IntegerTypeVariant::I16)]
    #[test_case("-32768", IntegerTypeVariant::I16)]
    #[test_case("2147483647", IntegerTypeVariant::I32)]
    #[test_case("-2147483648", IntegerTypeVariant::I32)]
    #[test_case("9223372036854775807", IntegerTypeVariant::I64)]
    #[test_case("-9223372036854775808", IntegerTypeVariant::I64)]
    #[test_case(
        "170141183460469231731687303715884105727",
        IntegerTypeVariant::I128
    )]
    #[test_case(
        "-170141183460469231731687303715884105728",
        IntegerTypeVariant::I128
    )]
    #[test_case(
        "999999999999999999999999999999999999999999",
        IntegerTypeVariant::IBig
    )]
    #[test_case("255", IntegerTypeVariant::U8)]
    #[test_case("65535", IntegerTypeVariant::U16)]
    #[test_case("4294967295", IntegerTypeVariant::U32)]
    #[test_case("18446744073709551615", IntegerTypeVariant::U64)]
    #[test_case(
        "340282366920938463463374607431768211455",
        IntegerTypeVariant::U128
    )]
    fn typed_integer_accepts_string_numbers(
        v: &str,
        variant: IntegerTypeVariant,
    ) {
        let actual = deserialize_value(int_variant(variant), json!(v)).unwrap();

        assert_eq!(
            actual,
            CoreValue::TypedInteger(
                TypedInteger::try_from_string_and_variant(v, variant).unwrap()
            )
        );
    }

    #[test_case("128", IntegerTypeVariant::I8)]
    #[test_case("-129", IntegerTypeVariant::I8)]
    #[test_case("256", IntegerTypeVariant::U8)]
    #[test_case("-1", IntegerTypeVariant::U8)]
    #[test_case("abc", IntegerTypeVariant::I32)]
    #[test_case("1.25", IntegerTypeVariant::I32)]
    fn typed_integer_rejects_invalid_string_numbers(
        v: &str,
        variant: IntegerTypeVariant,
    ) {
        let err =
            deserialize_value(int_variant(variant), json!(v)).unwrap_err();

        assert!(err.to_string().contains("failed to parse integer"), "{err}");
    }

    #[test]
    fn integer_number_rejects_non_integer_core_type() {
        let err = deserialize_value(base(CoreLibBaseTypeId::Text), json!(123))
            .unwrap_err();

        assert!(
            err.to_string()
                .contains("expected CoreValue of type Integer"),
            "{err}"
        );
    }

    #[test_case(json!(12), CoreLibBaseTypeId::Decimal; "integer number as decimal")]
    #[test_case(json!(-12), CoreLibBaseTypeId::Decimal; "negative integer number as decimal")]
    #[test_case(json!(12.5), CoreLibBaseTypeId::Decimal; "float number as decimal")]
    fn base_decimal_accepts_numbers(
        value: serde_json::Value,
        base_type: CoreLibBaseTypeId,
    ) {
        let actual = deserialize_value(base(base_type), value.clone()).unwrap();

        let expected = if let Some(v) = value.as_i64() {
            CoreValue::Decimal(
                Decimal::try_from_string(&v.to_string()).unwrap(),
            )
        } else {
            CoreValue::Decimal(Decimal::from(value.as_f64().unwrap()))
        };

        assert_eq!(actual, expected);
    }

    #[test_case(12_i64, DecimalTypeVariant::F32)]
    #[test_case(12_i64, DecimalTypeVariant::F64)]
    #[test_case(12_i64, DecimalTypeVariant::DBig)]
    fn decimal_variants_accept_integer_numbers(
        v: i64,
        variant: DecimalTypeVariant,
    ) {
        let actual = deserialize_value(dec_variant(variant), json!(v)).unwrap();

        let expected = match variant {
            DecimalTypeVariant::F32 => {
                CoreValue::TypedDecimal(TypedDecimal::F32((v as f32).into()))
            }
            DecimalTypeVariant::F64 => {
                CoreValue::TypedDecimal(TypedDecimal::F64((v as f64).into()))
            }
            DecimalTypeVariant::DBig => CoreValue::TypedDecimal(
                TypedDecimal::Decimal((v as f64).into()),
            ),
        };

        assert_eq!(actual, expected);
    }

    #[test_case(12.5_f64, DecimalTypeVariant::F32)]
    #[test_case(12.5_f64, DecimalTypeVariant::F64)]
    #[test_case(12.5_f64, DecimalTypeVariant::DBig)]
    fn decimal_variants_accept_float_numbers(
        v: f64,
        variant: DecimalTypeVariant,
    ) {
        let actual = deserialize_value(dec_variant(variant), json!(v)).unwrap();

        let expected = match variant {
            DecimalTypeVariant::F32 => {
                CoreValue::TypedDecimal(TypedDecimal::F32((v as f32).into()))
            }
            DecimalTypeVariant::F64 => {
                CoreValue::TypedDecimal(TypedDecimal::F64(v.into()))
            }
            DecimalTypeVariant::DBig => {
                CoreValue::TypedDecimal(TypedDecimal::Decimal(v.into()))
            }
        };

        assert_eq!(actual, expected);
    }

    #[test_case("0")]
    #[test_case("-1")]
    #[test_case("123.456")]
    #[test_case("999999999999999999999999999999.999")]
    fn base_decimal_accepts_string_numbers(v: &str) {
        let actual =
            deserialize_value(base(CoreLibBaseTypeId::Decimal), json!(v))
                .unwrap();

        assert_eq!(
            actual,
            CoreValue::Decimal(Decimal::try_from_string(v).unwrap())
        );
    }

    #[test_case("0", DecimalTypeVariant::F32)]
    #[test_case("-1.5", DecimalTypeVariant::F32)]
    #[test_case("123.456", DecimalTypeVariant::F64)]
    #[test_case("999999999999999999999999999999.999", DecimalTypeVariant::DBig)]
    fn typed_decimal_accepts_string_numbers(
        v: &str,
        variant: DecimalTypeVariant,
    ) {
        let actual = deserialize_value(dec_variant(variant), json!(v)).unwrap();

        assert_eq!(
            actual,
            CoreValue::TypedDecimal(
                TypedDecimal::try_from_string_and_variant(v, variant).unwrap()
            )
        );
    }

    #[test_case("abc")]
    #[test_case("")]
    #[test_case("--1")]
    fn base_decimal_rejects_invalid_string_numbers(v: &str) {
        let err = deserialize_value(base(CoreLibBaseTypeId::Decimal), json!(v))
            .unwrap_err();

        assert!(err.to_string().contains("failed to parse decimal"), "{err}");
    }

    #[test_case("abc", DecimalTypeVariant::F32)]
    #[test_case("", DecimalTypeVariant::F64)]
    #[test_case("--1", DecimalTypeVariant::DBig)]
    fn typed_decimal_rejects_invalid_string_numbers(
        v: &str,
        variant: DecimalTypeVariant,
    ) {
        let err =
            deserialize_value(dec_variant(variant), json!(v)).unwrap_err();

        assert!(err.to_string().contains("failed to parse decimal"), "{err}");
    }

    #[test]
    fn decimal_number_rejects_non_decimal_core_type() {
        let err =
            deserialize_value(base(CoreLibBaseTypeId::Boolean), json!(12.5))
                .unwrap_err();

        assert!(
            err.to_string()
                .contains("expected CoreValue of type Decimal"),
            "{err}"
        );
    }

    #[test_case("")]
    #[test_case("hello")]
    fn text_value_is_deserialized(v: &str) {
        let actual =
            deserialize_value(base(CoreLibBaseTypeId::Text), json!(v)).unwrap();

        assert_eq!(actual, CoreValue::Text(v.into()));
    }

    #[test]
    fn endpoint_value_is_deserialized_from_string() {
        let value = "@jonas";

        let actual =
            deserialize_value(base(CoreLibBaseTypeId::Endpoint), json!(value))
                .unwrap();

        assert_eq!(
            actual,
            CoreValue::Endpoint(Endpoint::from_str(value).unwrap())
        );
    }

    #[test_case("")]
    #[test_case("not an endpoint")]
    fn endpoint_rejects_invalid_string(v: &str) {
        let err =
            deserialize_value(base(CoreLibBaseTypeId::Endpoint), json!(v))
                .unwrap_err();

        assert!(
            err.to_string().contains("failed to parse endpoint"),
            "{err}"
        );
    }

    #[test]
    fn string_rejects_unexpected_core_type() {
        let err =
            deserialize_value(base(CoreLibBaseTypeId::Boolean), json!("true"))
                .unwrap_err();

        assert!(err.to_string().contains("unexpected"), "{err}");
    }

    #[test]
    fn null_value_is_deserialized() {
        let actual = deserialize_value(
            base(CoreLibBaseTypeId::Null),
            serde_json::Value::Null,
        )
        .unwrap();

        assert_eq!(actual, CoreValue::Null);
    }

    #[test]
    fn null_rejects_non_null_core_type() {
        let err = deserialize_value(
            base(CoreLibBaseTypeId::Text),
            serde_json::Value::Null,
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("expected CoreValue of type Unit"),
            "{err}"
        );
    }

    #[test]
    fn list_value_is_deserialized_from_sequence() {
        let actual = deserialize_value(
            base(CoreLibBaseTypeId::List),
            json!([1, "two", true]),
        )
        .unwrap();

        assert!(matches!(actual, CoreValue::List(_)));
    }

    #[test]
    fn range_value_is_deserialized_from_sequence() {
        let actual =
            deserialize_value(base(CoreLibBaseTypeId::Range), json!([1, 10]))
                .unwrap();

        assert!(matches!(actual, CoreValue::Range(_)));
    }

    #[test]
    fn map_value_is_deserialized_from_sequence() {
        let actual = deserialize_value(
            base(CoreLibBaseTypeId::Map),
            json!([["a", 1], ["b", 2]]),
        )
        .unwrap();

        assert!(matches!(actual, CoreValue::Map(_)));
    }

    #[test]
    fn sequence_rejects_non_sequence_core_type() {
        let err =
            deserialize_value(base(CoreLibBaseTypeId::Text), json!([1, 2, 3]))
                .unwrap_err();

        assert!(
            err.to_string()
                .contains("expected CoreValue of type List, Map, or Range"),
            "{err}"
        );
    }

    #[test]
    fn map_value_is_deserialized_from_object() {
        let actual = deserialize_value(
            base(CoreLibBaseTypeId::Map),
            json!({
                "a": 1,
                "b": true,
                "c": "text"
            }),
        )
        .unwrap();

        assert!(matches!(actual, CoreValue::Map(_)));
    }

    #[test]
    fn object_rejects_non_map_core_type() {
        let err = deserialize_value(
            base(CoreLibBaseTypeId::List),
            json!({
                "a": 1
            }),
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("expected CoreValue of type Map"),
            "{err}"
        );
    }
}
