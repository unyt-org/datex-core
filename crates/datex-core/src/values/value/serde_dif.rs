use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    libs::core::{core_lib_id::CoreLibIdIndex, type_id::CoreLibTypeId},
    prelude::*,
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        value::{
            Value,
            value_classification::{ValueClassification, ValueTag},
        },
    },
};
use core::fmt;
use num::ToPrimitive;
use serde::{
    Deserializer, Serialize, Serializer,
    de::{Error as DeError, MapAccess, Visitor},
    ser::{SerializeMap, SerializeTuple},
};
use crate::types::entities::entity_type_definition::dyn_deserialize::DynSeed;

impl<'ctx> SerdeContext<'ctx> {
    /// This method is used to serialize a value that can be represented directly depending on the flag set (e.g. a boolean or a text)
    /// or with a custom type definition (e.g. a nominal type).
    /// ## For no custom type:
    ///   true -> "true"
    ///   "Hello" -> "Hello"
    ///   42f64 -> 42
    ///   42f32 -> "42"
    /// ## For custom type:
    /// {custom_type: LiteralTypeDefinition::Integer(42), value: 42} -> [<core_lib_id>, <type_definition>, 42]
    pub(crate) fn serialize_core_value<Se, T>(
        &self,
        inner: &T,
        core_lib_type_id: CoreLibTypeId,
        serializer: Se,
        direct: bool,
    ) -> Result<Se::Ok, Se::Error>
    where
        T: Serialize + ?Sized,
        Se: Serializer,
    {
        if direct {
            return inner.serialize(serializer);
        }

        let index = CoreLibIdIndex::from(core_lib_type_id);
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&index.to_u16())?;
        tuple.serialize_element(inner)?;
        tuple.end()
    }

    /// This method is used to serialize a value that requires a context to be serialized (i.e. a value that implements [SerializeWithSerdeContext]).
    /// It will serialize the value as a tuple of [<core_lib_id>, <value>, <classification>], where the classification is optional.
    pub(crate) fn serialize_value_with_context<Se, T>(
        &self,
        inner: &T,
        core_lib_type_id: CoreLibTypeId,
        serializer: Se,
        direct: bool,
    ) -> Result<Se::Ok, Se::Error>
    where
        T: Sized,
        Se: Serializer,
        for<'a> T: SerializeWithSerdeContext,
    {
        if direct {
            return inner.serialize_with_ctx(self, serializer);
        }
        let index = CoreLibIdIndex::from(core_lib_type_id);
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&index.to_u16())?;
        tuple.serialize_element(&ValueWithSerdeContext::new(inner, self))?;
        tuple.end()
    }

    /// Deserializes into a [CoreValue] using the provided [CoreLibTypeId] to determine the type of the value.
    pub(crate) fn deserialize_core_value<'de, D>(
        &self,
        deserializer: D,
    ) -> Result<CoreValue, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserialize_ctx = DeserializeSerdeContext::<CoreValue>::new(self);
        deserializer.deserialize_any(deserialize_ctx)
    }
}

/// Serialization for [Value].
impl SerializeWithSerdeContext for Value {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Native(native) => native.serialize_with_ctx(ctx, serializer),
            Value::Core(core) => core.serialize_with_ctx(ctx, serializer),
        }
    }
}

impl SerializeWithSerdeContext for (&CoreValue, &Option<ValueTag>) {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // if no tag, just serialize the core value
        if self.1.is_none() {
            self.0.serialize_with_ctx(ctx, serializer)
        }
        // else serialize as a map with "t" and "v" keys
        else {
            let mut map = serializer.serialize_map(Some(3))?;
            map.serialize_entry("_", "t")?;
            map.serialize_entry("t", &self.1)?;
            map.serialize_entry(
                "v",
                &ValueWithSerdeContext::new(&self.0, ctx),
            )?;
            map.end()
        }
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Value {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DeserializeSerdeContext::<Value>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, Value> {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a value, which can be a direct core value (e.g. boolean, text) or a complex value with a custom type definition")
    }

    /// default mapping for unit: null
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(Value::new(self.cast::<CoreValue>().visit_unit()?))
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
        Ok(Value::new(self.cast::<CoreValue>().visit_bool(v)?))
    }

    /// default mapping for string: text
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(Value::new(self.cast::<CoreValue>().visit_str(v)?))
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
        Ok(Value::new(self.cast::<CoreValue>().visit_f64(v)?))
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

    /// mapping for [tag?, [core_lib_type_id, core_value]]
    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        Ok(Value::Core(
            self.cast::<CoreValueWithClassification>().visit_seq(seq)?,
        ))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut classification = ValueClassification::new_unclassified();

        // loop over keys until we find "c" or "v" or reach the end of the map
        while let Some(key) = map.next_key::<String>()? {
            // if "_" indicator is present, skip it and continue to the next key
            // not relevant on this level
            if key == "_" {
                let _: serde_json::Value = map.next_value()?;
                continue;
            }
            // get the classification first if it exists
            else if key == "c" {
                classification = map.next_value_seed(self.cast::<ValueClassification>())?;
                // resolve native value based on entity classification
                if let Some(entity_type) = &classification.entity_type &&
                    let Some(deserialize_fn) = entity_type.entity_definition().deserialize_from_dif {
                        // next key must be "v" for the value, otherwise error
                        let key = map.next_key::<String>()?.ok_or_else(|| DeError::custom("Expected key 'v' not found in map"))?;
                        if key != "v" {
                            return Err(DeError::custom(format!(
                                "Expected key 'v' not found in map, found key '{key}' instead"
                            )));
                        }
                        // deserialize via the entity type deserialize function
                        let value = map.next_value_seed(DynSeed { f: deserialize_fn, ctx: self.ctx })?;
                        return Ok(value);
                }
            }
            // Normal, non-native core value:
            else if key == "v" {
                let mut value = map.next_value_seed(self.cast::<CoreValueWithClassification>())?;
                value.classification = value.classification.merge(classification);
                return Ok(Value::Core(value));
            } else {
                return Err(DeError::custom(format!(
                    "Unexpected key in map: {key}"
                )));
            }
        }
        Err(DeError::custom("Expected key 'v' not found in map"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        libs::core::{
            core_lib_id::CoreLibIdIndex,
            type_id::{CoreLibBaseTypeId, CoreLibVariantTypeId},
        },
        runtime::cache::shared_values_cache::SharedValuesCache,
        values::{
            core_value::CoreValue,
            core_values::{
                decimal::typed_decimal::{DecimalTypeVariant, TypedDecimal},
                endpoint::Endpoint,
                instant::Instant,
                integer::{Integer, typed_integer::IntegerTypeVariant},
                map::Map,
            },
            value_container::ValueContainer,
        },
    };
    use core::{cell::RefCell, str::FromStr};
    use test_case::test_case;

    #[test]
    fn endpoint_serialization() {
        let endpoint = Endpoint::from_str("@jonas").unwrap();
        let value = Value::new(CoreValue::Endpoint(endpoint.clone()));
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);
        let serialized = context.serialize_to_json(&value);
        assert_eq!(
            serialized,
            format!(
                r#"[{},"{}"]"#,
                CoreLibIdIndex::from(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Endpoint
                )),
                endpoint
            )
        );
    }

    #[test]
    fn serialize_map() {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        // { endpoint: "@jonas" } -> [<map-idx>, { endpoint: [<endpoint-idx>, "@jonas"] }]
        let value = Value::from(CoreValue::Map(
            Map::new_structural(vec![(
                "endpoint".into(),
                ValueContainer::Local(Value::from(
                    Endpoint::from_str("@jonas").unwrap(),
                )),
            )]),
        ));
        let serialized = context.serialize_to_json(&value);
        assert_eq!(
            serialized,
            format!(
                r#"[{},{{"endpoint":[{},"@jonas"]}}]"#,
                CoreLibIdIndex::from(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Map
                )),
                CoreLibIdIndex::from(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Endpoint
                ))
            )
        );

        // { "endpoint": "@jonas" } -> [<map-idx>, [[<endpoint-idx>, "@jonas"]]]
        let value = Value::from(CoreValue::Map(Map::new_structural(vec![(
            "endpoint".into(),
            Value::from(Endpoint::from_str("@jonas").unwrap()).into(),
        )])));
        let serialized = context.serialize_to_json(&value);
        assert_eq!(
            serialized,
            format!(
                r#"[{},[["endpoint",[{},"@jonas"]]]]"#,
                CoreLibIdIndex::from(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Map
                )),
                CoreLibIdIndex::from(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Endpoint
                ))
            )
        );
    }

    #[test]
    fn default_representation() {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        // text
        let value = Value::from(CoreValue::Text("Hello, world!".into()));
        let serialized = context.serialize_to_json(&value);
        assert_eq!(serialized, r#""Hello, world!""#);

        // decimal f64
        let value = Value::from(CoreValue::TypedDecimal(TypedDecimal::F64(
            5.14f64.into(),
        )));
        let serialized = context.serialize_to_json(&value);
        assert_eq!(serialized, r#"5.14"#);

        // boolean
        let value = Value::from(CoreValue::Boolean(true.into()));
        let serialized = context.serialize_to_json(&value);
        assert_eq!(serialized, r#"true"#);
    }

    #[test]
    fn non_default_representation() {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        // f32
        let value = Value::from(CoreValue::TypedDecimal(TypedDecimal::F32(
            5.14f32.into(),
        )));
        let serialized = context.serialize_to_json(&value);

        assert_eq!(
            serialized,
            format!(
                r#"[{},5.14]"#,
                CoreLibIdIndex::from(CoreLibTypeId::Variant(
                    CoreLibVariantTypeId::Decimal(DecimalTypeVariant::F32)
                ))
            )
        );

        // integer
        let value = Value::from(CoreValue::Integer(Integer::new(42)));
        let serialized = context.serialize_to_json(&value);
        assert_eq!(
            serialized,
            format!(
                r#"[{},"42"]"#,
                CoreLibIdIndex::from(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Integer
                ))
            )
        );

        // typed integer
        let value = Value::from(CoreValue::TypedInteger(42u8.into()));
        let serialized = context.serialize_to_json(&value);
        assert_eq!(
            serialized,
            format!(
                r#"[{},42]"#,
                CoreLibIdIndex::from(CoreLibTypeId::Variant(
                    CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)
                ))
            )
        );
    }

    #[test_case(
        CoreValue::Text("Hello, world!".into()) ; "text"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F32(5.14f32.into())) ; "decimal f32"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F64(f64::NAN.into())) ; "nan f64"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F32(f32::NAN.into())) ; "nan f32"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F32(f32::INFINITY.into())) ; "inf f32"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F64(f64::INFINITY.into())) ; "inf f64"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F64(5.14f64.into())) ; "decimal f64"
    )]
    #[test_case(
        CoreValue::Boolean(true.into()) ; "boolean"
    )]
    #[test_case(
        CoreValue::TypedInteger(42u8.into()) ; "typed integer u8"
    )]
    #[test_case(
        CoreValue::Endpoint(Endpoint::from_str("@jonas").unwrap()) ; "endpoint"
    )]
    #[test_case(
        CoreValue::Map(Map::new_structural(vec![(
            "endpoint".into(),
            ValueContainer::Local(Value::from(Endpoint::from_str("@jonas").unwrap())),
        )])) ; "map with string keys"
    )]
    #[test_case(
        CoreValue::Null ; "null"
    )]
    #[test_case(
        CoreValue::Integer(Integer::new(42)) ; "base integer"
    )]
    #[test_case(
        CoreValue::TypedInteger((-42i8).into()) ; "typed integer i8"
    )]
    #[test_case(
        CoreValue::TypedInteger(42u16.into()) ; "typed integer u16"
    )]
    #[test_case(
        CoreValue::TypedInteger(42u32.into()) ; "typed integer u32"
    )]
    #[test_case(
        CoreValue::TypedInteger(42u64.into()) ; "typed integer u64"
    )]
    #[test_case(
        CoreValue::TypedInteger(42u128.into()) ; "typed integer u128"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F64(f64::NEG_INFINITY.into())) ; "negative inf f64"
    )]
    #[test_case(
        CoreValue::TypedDecimal(TypedDecimal::F32(f32::NEG_INFINITY.into())) ; "negative inf f32"
    )]
    #[test_case(
        CoreValue::Instant(Instant::now()) ; "instant"
    )]
    fn roundtrip_no_custom_type(value: CoreValue) {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        let value = Value::from(value);
        let serialized = context.serialize_to_json(&value);
        let deserialized: Value =
            context.try_deserialize_from_json(&serialized).unwrap();
        assert_eq!(deserialized, value);
    }
}
