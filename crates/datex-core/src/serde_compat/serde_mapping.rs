use crate::{
    prelude::*,
    values::{
        core_value::CoreValue,
        core_values::{
            decimal::typed_decimal::TypedDecimal,
            integer::typed_integer::TypedInteger, list::List, map::Map,
        },
        value::{Value, borrowed_value::BorrowedValue},
        value_container::{
            ValueContainer, borrowed_value_container::BorrowedValueContainer,
        },
    },
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::Visitor,
    ser::{SerializeMap, SerializeSeq},
};
use crate::values::core_values::text::Text;

impl Serialize for ValueContainer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ValueContainer::Local(value) => value.serialize(serializer),
            ValueContainer::Shared(_) => Err(serde::ser::Error::custom(
                "Cannot serialize shared value container",
            )),
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self {
            Value::Core(value) => value.inner.serialize(serializer),
            Value::Native(_native) => {
                unreachable!("Cannot serialize native value without ctx")
            }
        }
    }
}

impl Serialize for CoreValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            CoreValue::Null => serializer.serialize_unit(),
            CoreValue::Boolean(v) => serializer.serialize_bool(v.as_bool()),
            CoreValue::Text(v) => serializer.serialize_str(v.as_str()),
            CoreValue::TypedInteger(typed_integer) => match typed_integer {
                TypedInteger::U8(v) => serializer.serialize_u8(*v),
                TypedInteger::U16(v) => serializer.serialize_u16(*v),
                TypedInteger::U32(v) => serializer.serialize_u32(*v),
                TypedInteger::U64(v) => serializer.serialize_u64(*v),
                TypedInteger::U128(v) => serializer.serialize_u128(*v),
                TypedInteger::I8(v) => serializer.serialize_i8(*v),
                TypedInteger::I16(v) => serializer.serialize_i16(*v),
                TypedInteger::I32(v) => serializer.serialize_i32(*v),
                TypedInteger::I64(v) => serializer.serialize_i64(*v),
                TypedInteger::I128(v) => serializer.serialize_i128(*v),
                TypedInteger::IBig(_) => Err(serde::ser::Error::custom(
                    "Cannot serialize IBig typed integer",
                )),
            },
            CoreValue::TypedDecimal(typed_decimal) => match typed_decimal {
                TypedDecimal::F32(v) => serializer.serialize_f32(v.0),
                TypedDecimal::F64(v) => serializer.serialize_f64(v.0),
                TypedDecimal::Decimal(_) => Err(serde::ser::Error::custom(
                    "Cannot serialize Decimal typed decimal",
                )),
            },
            CoreValue::List(list) => {
                let mut list_state =
                    serializer.serialize_seq(Some(list.len() as usize))?;
                for item in list {
                    list_state.serialize_element(item)?;
                }
                list_state.end()
            }
            CoreValue::Map(map) => {
                let mut map_state =
                    serializer.serialize_map(Some(map.size()))?;
                for (key, value) in map.iter() {
                    map_state.serialize_key(&key)?;
                    map_state.serialize_value(value)?;
                }
                map_state.end()
            }
            val => Err(serde::ser::Error::custom(format!(
                "Cannot serialize value: {:?}",
                val
            ))),
        }
    }
}

impl Serialize for BorrowedValueContainer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BorrowedValueContainer::Local(value) => value.serialize(serializer),
            BorrowedValueContainer::Shared(_) => {
                Err(serde::ser::Error::custom(
                    "Cannot serialize shared value container",
                ))
            }
        }
    }
}

impl Serialize for BorrowedValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BorrowedValue::Core(value) => value
                .clone_to_core_value_with_classification()
                .serialize(serializer),
            BorrowedValue::Native(_native) => {
                unreachable!("Cannot serialize native value without ctx")
            }
        }
    }
}

impl<'de> Deserialize<'de> for ValueContainer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(ValueContainer::Local(value))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let core_value = CoreValue::deserialize(deserializer)?;
        Ok(Value::from(core_value))
    }
}

impl<'de> Deserialize<'de> for CoreValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CoreValueVisitor;

        impl<'de> Visitor<'de> for CoreValueVisitor {
            type Value = CoreValue;

            fn expecting(
                &self,
                formatter: &mut core::fmt::Formatter,
            ) -> core::fmt::Result {
                formatter.write_str("a valid DATEX core value")
            }

            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::Boolean(v.into()))
            }

            fn visit_i8<E: serde::de::Error>(
                self,
                v: i8,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::I8(v)))
            }

            fn visit_i16<E: serde::de::Error>(
                self,
                v: i16,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::I16(v)))
            }

            fn visit_i32<E: serde::de::Error>(
                self,
                v: i32,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::I32(v)))
            }

            fn visit_i64<E: serde::de::Error>(
                self,
                v: i64,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::I64(v)))
            }

            fn visit_i128<E: serde::de::Error>(
                self,
                v: i128,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::I128(v)))
            }

            fn visit_u8<E: serde::de::Error>(
                self,
                v: u8,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::U8(v)))
            }

            fn visit_u16<E: serde::de::Error>(
                self,
                v: u16,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::U16(v)))
            }

            fn visit_u32<E: serde::de::Error>(
                self,
                v: u32,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::U32(v)))
            }

            fn visit_u64<E: serde::de::Error>(
                self,
                v: u64,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::U64(v)))
            }

            fn visit_u128<E: serde::de::Error>(
                self,
                v: u128,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedInteger(TypedInteger::U128(v)))
            }

            fn visit_f32<E: serde::de::Error>(
                self,
                v: f32,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedDecimal(TypedDecimal::F32(v.into())))
            }

            fn visit_f64<E: serde::de::Error>(
                self,
                v: f64,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::TypedDecimal(TypedDecimal::F64(v.into())))
            }

            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::Text(Text::String(v.to_string())))
            }

            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> Result<Self::Value, E> {
                Ok(CoreValue::Text(v.into()))
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(CoreValue::Null)
            }

            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                CoreValue::deserialize(deserializer)
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(CoreValue::Null)
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut list = match seq.size_hint() {
                    Some(n) => List::with_capacity(n as u32),
                    None => List::default(),
                };
                while let Some(item) = seq.next_element::<ValueContainer>()? {
                    list.push(item);
                }
                Ok(CoreValue::List(list))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Self::Value, A::Error> {
                let mut map = Map::default();
                while let Some(key) = access.next_key::<String>()? {
                    let value: ValueContainer = access.next_value()?;
                    map.set_unchecked(ValueContainer::from(key), value);
                }
                Ok(CoreValue::Map(map))
            }
        }

        deserializer.deserialize_any(CoreValueVisitor)
    }
}
