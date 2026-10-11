use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        value::value_classification::{ValueClassification, ValueTag},
    },
};
use core::fmt;
use num_traits::ToPrimitive;
use serde::{
    Deserializer, Serialize, Serializer,
    de::{Error, MapAccess, Visitor},
    ser::SerializeMap,
};

impl SerializeWithSerdeContext for CoreValueWithClassification {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let classification = &self.classification;

        // if tag, serialize as a map with "t" and "v" keys
        if let Some(tag) = &classification.tag {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("t", tag)?;
            map.serialize_entry(
                "v",
                &ValueWithSerdeContext::new(&self.inner, ctx),
            )?;
            map.end()
        }
        // if no tag, just serialize the inner value with the context
        else {
            ValueWithSerdeContext::new(&self.inner, ctx).serialize(serializer)
        }
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for CoreValueWithClassification {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DeserializeSerdeContext::<
            CoreValueWithClassification,
        >::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, CoreValueWithClassification>
{
    type Value = CoreValueWithClassification;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a value with an optional tag, which can be a direct core value (e.g. boolean, text) or a complex value with a custom type definition")
    }

    /// default mapping for bool: boolean
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(CoreValueWithClassification::new(
            self.cast::<CoreValue>().visit_bool(v)?,
        ))
    }

    // default mapping for integers: decimal/f64 (with a check for overflow)
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            Error::custom(format!("i64 value {v} is too large to fit into f64"))
        })?)
    }
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            Error::custom(format!(
                "i128 value {v} is too large to fit into f64"
            ))
        })?)
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            Error::custom(format!("u64 value {v} is too large to fit into f64"))
        })?)
    }
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v.to_f64().ok_or_else(|| {
            Error::custom(format!(
                "u128 value {v} is too large to fit into f64"
            ))
        })?)
    }
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_f64(v as f64)
    }
    /// default mapping for f64: decimal/f64
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(CoreValueWithClassification::new(
            self.cast::<CoreValue>().visit_f64(v)?,
        ))
    }
    /// default mapping for string: text
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(CoreValueWithClassification::new(
            self.cast::<CoreValue>().visit_str(v)?,
        ))
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(&v)
    }
    /// default mapping for none: null
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_unit()
    }
    // fallback for direct core values without a tag
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(CoreValueWithClassification::new(
            self.cast::<CoreValue>().visit_unit()?,
        ))
    }
    /// mapping for [core_lib_type_id, core_value]
    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        Ok(CoreValueWithClassification::new(
            self.cast::<CoreValue>().visit_seq(seq)?,
        ))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut core_value: Option<CoreValue> = None;
        let mut tag: Option<ValueTag> = None;

        while let Some(field) = map.next_key::<String>()? {
            match field.as_str() {
                "v" => {
                    core_value =
                        Some(map.next_value_seed(self.cast::<CoreValue>())?);
                }
                "t" => {
                    tag = Some(map.next_value()?);
                }
                _ => {
                    return Err(A::Error::custom(format!(
                        "Unexpected key for DIF value with tag: {}",
                        field
                    )));
                }
            }
        }

        match core_value {
            Some(core_value) => {
                Ok(CoreValueWithClassification::new_with_classification(
                    core_value,
                    ValueClassification::new_with_maybe_tag(tag),
                ))
            }
            None => Err(A::Error::custom(
                "Expected a 'v' key for the DIF value with tag",
            )),
        }
    }
}
