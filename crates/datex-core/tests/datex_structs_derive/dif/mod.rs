use core::cell::RefCell;

use datex_core::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    runtime::cache::shared_values_cache::SharedValuesCache,
};
use serde::de::IntoDeserializer;

mod r#enum;
mod r#struct;

/// TODO: general helper function in datex_core
pub fn serialize_to_json_value(
    context: &SerdeContext,
    value: impl SerializeWithSerdeContext,
) -> serde_json::Value {
    let serializer = serde_json::value::Serializer;
    value.serialize_with_ctx(context, serializer).unwrap()
}

/// Generates a serialized DIF JSON string from a value using a default SerdeContext and SharedValuesCache.
pub fn serialize_to_json_value_default_ctx(
    value: impl SerializeWithSerdeContext,
) -> serde_json::Value {
    let cache = RefCell::new(SharedValuesCache::default());
    let context = SerdeContext::new(&cache);
    serialize_to_json_value(&context, value)
}

pub fn deserialize_from_dif<'de, T: DeserializeWithSerdeContext<'de>>(
    context: &SerdeContext<'_>,
    json_value: &'de serde_json::Value,
) -> T {
    let deserializer = json_value.into_deserializer();
    T::deserialize_with_ctx(context, deserializer).unwrap()
}

pub fn deserialize_from_dif_default_ctx<
    'de,
    T: DeserializeWithSerdeContext<'de>,
>(
    json_value: &'de serde_json::Value,
) -> T {
    let cache = RefCell::new(SharedValuesCache::default());
    let context = SerdeContext::new(&cache);
    deserialize_from_dif(&context, json_value)
}
