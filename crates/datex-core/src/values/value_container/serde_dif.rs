use crate::{
    shared_values::SharedContainer,
    values::{value::Value, value_container::ValueContainer},
};
use serde::{Serializer, ser::SerializeSeq};

use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        pointer_address::PointerAddressWithOwnership,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    prelude::*,
    values::core_value_with_classification::CoreValueWithClassification,
};
use core::fmt;
use serde::{
    Deserializer,
    de::{MapAccess, Visitor},
};

pub const DIF_INDICATOR_KEY: &str = "_";

/// Deserialization for [ValueContainer] using a [SerdeContext] to provide access to the memory during deserialization.
impl<'de> DeserializeWithSerdeContext<'de> for ValueContainer {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        d: D,
    ) -> Result<ValueContainer, D::Error> {
        d.deserialize_any(DeserializeSerdeContext::<ValueContainer>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, ValueContainer>
{
    type Value = ValueContainer;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a map for a shared container of any DIF value representation for local values")
    }

    fn visit_bool<E>(self, v: bool) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_bool(v)?))
    }

    fn visit_i64<E>(self, v: i64) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_i64(v)?))
    }

    fn visit_u64<E>(self, v: u64) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_u64(v)?))
    }

    fn visit_f64<E>(self, v: f64) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_f64(v)?))
    }
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_i16(v)?))
    }
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_i32(v)?))
    }
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_u128(v)?))
    }
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_i8(v)?))
    }
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_u16(v)?))
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_u32(v)?))
    }
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_u8(v)?))
    }
    fn visit_str<E>(self, v: &str) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_str(v)?))
    }
    fn visit_string<E>(self, v: String) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_string(v)?))
    }

    fn visit_none<E>(self) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_none()?))
    }

    fn visit_unit<E>(self) -> Result<ValueContainer, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_unit()?))
    }

    fn visit_some<D>(self, d: D) -> Result<ValueContainer, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_some(d)?))
    }

    fn visit_seq<A>(self, seq: A) -> Result<ValueContainer, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_seq(seq)?))
    }

    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_i128(v)?))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueContainer::Local(self.cast::<Value>().visit_f32(v)?))
    }

    // map => shared container { $: pointer address }
    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> Result<ValueContainer, A::Error> {
        let Some(key) = map.next_key::<String>()? else {
            return Err(serde::de::Error::custom(format!(
                "expected an indicator key '{DIF_INDICATOR_KEY}', but the map is empty"
            )));
        };

        if key != DIF_INDICATOR_KEY {
            return Err(serde::de::Error::custom(format!(
                "unexpected key {key:?}, expected an indicator key '{DIF_INDICATOR_KEY}'"
            )));
        }

        let indicator: String = map.next_value()?;

        // if indicator key is "c", then the value is a classified value, and we need to deserialize Value
        if indicator == "c" {
            let value = self.cast::<Value>().visit_map(map)?;
            Ok(ValueContainer::Local(value))
        }
        // if the indicator key is "t", this is a tagged value, and we need to deserialize Value
        else if indicator == "t" {
            let value =
                self.cast::<CoreValueWithClassification>().visit_map(map)?;
            Ok(ValueContainer::Local(Value::Core(value)))
        } else {
            let pointer_address_with_ownership =
                PointerAddressWithOwnership::try_from(indicator.as_str()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Failed to parse pointer address from indicator value '{indicator}': {e}"
                    ))
                })?;
            let shared = SharedContainer::resolve_with_serde_context(
                pointer_address_with_ownership,
                self.ctx,
            )
            .map_err(|e| {
                serde::de::Error::custom(format!(
                    "Failed to retrieve shared container from cache: {}",
                    e
                ))
            })?;

            if let Some(extra_key) = map.next_key::<String>()? {
                return Err(serde::de::Error::custom(format!(
                    "unexpected extra key {extra_key:?}, expected only an indicator key '{DIF_INDICATOR_KEY}'"
                )));
            }

            Ok(ValueContainer::Shared(shared))
        }
    }
}

impl SerializeWithSerdeContext for ValueContainer {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ValueContainer::Shared(shared) => {
                use serde::ser::SerializeMap;
                let pointer = unsafe { ctx.pointer_string(shared) };

                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(DIF_INDICATOR_KEY, &pointer)?;
                map.end()
            }
            ValueContainer::Local(local) => {
                local.serialize_with_ctx(ctx, serializer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        libs::core::{core_lib_id::CoreLibIdIndex, type_id::CoreLibBaseTypeId},
        runtime::{
            cache::shared_values_cache::SharedValuesCache,
            pointer_address_provider::SelfOwnedPointerAddressProvider,
        },
        shared_values::{
            OwnedSharedContainer, PointerAddress, SelfOwnedPointerAddress,
            SharedContainerMutability,
        },
        values::{core_value::CoreValue, core_values::list::List},
    };
    use core::cell::RefCell;

    #[test]
    fn pointer_address() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        let value = ValueContainer::Shared(SharedContainer::Owned(
            OwnedSharedContainer::new_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Mutable,
                &mut provider,
            ),
        ));
        let serialized = context.serialize_to_json(&value);

        // we expect the JSON to be of the form { "$": "<address>" }
        assert!(
            serialized.starts_with(r#"{"$":""#)
                && serialized.ends_with(r#""}"#)
        );

        let deserialized =
            context.try_deserialize_from_json(&serialized).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn owned() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        let value = ValueContainer::Shared(SharedContainer::Owned(
            OwnedSharedContainer::new_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Mutable,
                &mut provider,
            ),
        ));
        let serialized = context.serialize_to_json(&value);

        let address_string =
            serde_json::from_str::<serde_json::Value>(&serialized)
                .unwrap()
                .get(DIF_INDICATOR_KEY)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
        let addr = PointerAddress::SelfOwned(
            SelfOwnedPointerAddress::try_from(address_string).unwrap(),
        );
        let container =
            cache.borrow_mut().try_take_owned_shared_container(&addr);
        assert!(container.is_ok());
    }

    #[test]
    fn referenced() {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        let owned_container =
            OwnedSharedContainer::new_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Mutable,
                &mut SelfOwnedPointerAddressProvider::default(),
            );
        let referenced_container = SharedContainer::Referenced(
            owned_container.derive_immutable_reference(),
        );
        cache
            .borrow_mut()
            .store_shared_container(SharedContainer::Referenced(
                owned_container.derive_immutable_reference(),
            ));
        let pointer_address = referenced_container.pointer_address();
        let value = ValueContainer::Shared(referenced_container);
        let json = context.serialize_to_json(&value);
        assert_eq!(json, format!(r#"{{"$":"'{}"}}"#, pointer_address));

        let outer: ValueContainer =
            context.try_deserialize_from_json(&json).unwrap();

        assert_eq!(
            outer,
            ValueContainer::Shared(SharedContainer::Referenced(
                owned_container.derive_immutable_reference(),
            ))
        );
    }

    #[test]
    fn deserialize_nested_pointer_address() {
        let cache = RefCell::new(SharedValuesCache::default());
        let context = SerdeContext::new(&cache);

        let owned_container =
            OwnedSharedContainer::new_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Mutable,
                &mut SelfOwnedPointerAddressProvider::default(),
            );
        let referenced_container = SharedContainer::Referenced(
            owned_container.derive_immutable_reference(),
        );
        let pointer_address = referenced_container.pointer_address();
        cache
            .borrow_mut()
            .store_shared_container(referenced_container);
        let json = format!(
            r#"[{},[{{"_":"'{}"}}]]"#,
            CoreLibIdIndex::from(CoreLibBaseTypeId::List),
            pointer_address
        );
        let outer: ValueContainer =
            context.try_deserialize_from_json(&json).unwrap();

        assert_eq!(
            outer,
            ValueContainer::Local(Value::from(CoreValue::List(List::from(
                vec![ValueContainer::Shared(SharedContainer::Referenced(
                    owned_container.derive_immutable_reference(),
                ))]
            ))))
        );
    }
}
