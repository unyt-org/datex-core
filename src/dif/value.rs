use crate::dif::DIFConvertible;
use crate::dif::{
    representation::DIFValueRepresentation, r#type::DIFTypeContainer,
};
use crate::libs::core::CoreLibPointerId;
use crate::types::type_container::TypeContainer;
use crate::values::core_values::decimal::typed_decimal::{
    DecimalTypeVariant, TypedDecimal,
};
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::map::MapKey;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use datex_core::runtime::memory::Memory;
use datex_core::values::core_value::CoreValue;
use log::info;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Debug)]
pub struct DIFReferenceNotFoundError;

/// Represents a value in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFValue {
    pub value: DIFValueRepresentation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<DIFTypeContainer>,
}
impl DIFConvertible for DIFValue {}

impl DIFValue {
    /// Converts the DIFValue into a Value, resolving references using the provided memory.
    /// Returns an error if a reference cannot be resolved.
    pub fn to_value(
        self,
        memory: &RefCell<Memory>,
    ) -> Result<Value, DIFReferenceNotFoundError> {
        Ok(if let Some(r#type) = &self.r#type {
            self.value.to_value_with_type(r#type, memory)?
        } else {
            self.value.to_default_value(memory)?
        })
    }
}

impl DIFValue {
    pub fn new(
        value: DIFValueRepresentation,
        r#type: Option<impl Into<DIFTypeContainer>>,
    ) -> Self {
        DIFValue {
            value,
            r#type: r#type.map(Into::into),
        }
    }
    pub fn as_container(&self) -> DIFValueContainer {
        DIFValueContainer::from(self.clone())
    }
}

impl From<DIFValueRepresentation> for DIFValue {
    fn from(value: DIFValueRepresentation) -> Self {
        DIFValue {
            value,
            r#type: None,
        }
    }
}

/// Holder for either a value or a reference to a value in DIF
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DIFValueContainer {
    Value(DIFValue),
    Reference(PointerAddress),
}
impl DIFConvertible for DIFValueContainer {}

impl DIFValueContainer {
    /// Converts the DIFValueContainer into a ValueContainer, resolving references using the provided memory.
    /// Returns an error if a reference cannot be resolved.
    pub fn to_value_container(
        self,
        memory: &RefCell<Memory>,
    ) -> Result<ValueContainer, DIFReferenceNotFoundError> {
        Ok(match self {
            DIFValueContainer::Reference(address) => ValueContainer::Reference(
                memory
                    .borrow()
                    .get_reference(&address)
                    .ok_or(DIFReferenceNotFoundError)?
                    .clone(),
            ),
            DIFValueContainer::Value(v) => {
                ValueContainer::Value(v.to_value(memory)?)
            }
        })
    }
}

impl From<DIFValue> for DIFValueContainer {
    fn from(value: DIFValue) -> Self {
        DIFValueContainer::Value(value)
    }
}
impl From<&DIFValue> for DIFValueContainer {
    fn from(value: &DIFValue) -> Self {
        DIFValueContainer::Value(value.clone())
    }
}
impl From<PointerAddress> for DIFValueContainer {
    fn from(ptr: PointerAddress) -> Self {
        DIFValueContainer::Reference(ptr)
    }
}

impl DIFValueContainer {
    pub fn from_value_container(
        value_container: &ValueContainer,
        memory: &RefCell<Memory>,
    ) -> Self {
        match value_container {
            ValueContainer::Reference(reference) => {
                // get pointer address, if not present register the reference in memory
                let address = reference.ensure_pointer_address(memory);
                DIFValueContainer::Reference(address)
            }
            ValueContainer::Value(value) => {
                DIFValueContainer::Value(DIFValue::from_value(value, memory))
            }
        }
    }
}

impl DIFValue {
    fn from_value(value: &Value, memory: &RefCell<Memory>) -> Self {
        let core_value = &value.inner;

        let dif_core_value = match core_value {
            CoreValue::Type(ty) => todo!("Type value not supported in DIF"),
            CoreValue::Null => DIFValueRepresentation::Null,
            CoreValue::Boolean(bool) => DIFValueRepresentation::Boolean(bool.0),
            CoreValue::Integer(integer) => {
                // TODO: optimize this and pass as integer if in range
                DIFValueRepresentation::String(integer.to_string())
            }
            CoreValue::TypedInteger(integer) => {
                match integer {
                    TypedInteger::I8(i) => {
                        DIFValueRepresentation::Number(*i as f64)
                    }
                    TypedInteger::U8(u) => {
                        DIFValueRepresentation::Number(*u as f64)
                    }
                    TypedInteger::I16(i) => {
                        DIFValueRepresentation::Number(*i as f64)
                    }
                    TypedInteger::U16(u) => {
                        DIFValueRepresentation::Number(*u as f64)
                    }
                    TypedInteger::I32(i) => {
                        DIFValueRepresentation::Number(*i as f64)
                    }
                    TypedInteger::U32(u) => {
                        DIFValueRepresentation::Number(*u as f64)
                    }
                    // i64 and above are serialized as strings in DIF
                    TypedInteger::I64(i) => {
                        DIFValueRepresentation::String(i.to_string())
                    }
                    TypedInteger::U64(u) => {
                        DIFValueRepresentation::String(u.to_string())
                    }
                    TypedInteger::I128(i) => {
                        DIFValueRepresentation::String(i.to_string())
                    }
                    TypedInteger::U128(u) => {
                        DIFValueRepresentation::String(u.to_string())
                    }
                    TypedInteger::Big(i) => {
                        DIFValueRepresentation::String(i.to_string())
                    }
                }
            }
            CoreValue::Decimal(decimal) => {
                // TODO: optimize this and pass as decimal if in range
                DIFValueRepresentation::String(decimal.to_string())
            }
            CoreValue::TypedDecimal(decimal) => match decimal {
                TypedDecimal::F32(f) => {
                    DIFValueRepresentation::Number(f.0 as f64)
                }
                TypedDecimal::F64(f) => DIFValueRepresentation::Number(f.0),
                TypedDecimal::Decimal(bd) => {
                    DIFValueRepresentation::String(bd.to_string())
                }
            },
            CoreValue::Text(text) => {
                DIFValueRepresentation::String(text.0.clone())
            }
            CoreValue::Endpoint(endpoint) => {
                DIFValueRepresentation::String(endpoint.to_string())
            }
            CoreValue::List(list) => DIFValueRepresentation::Array(
                list.iter()
                    .map(|v| DIFValueContainer::from_value_container(v, memory))
                    .collect(),
            ),
            CoreValue::Map(map) => DIFValueRepresentation::Map(
                map.into_iter()
                    .map(|(k, v)| {
                        (
                            match k {
                                MapKey::Text(text_key) => {
                                    DIFValueContainer::Value(
                                        DIFValueRepresentation::String(
                                            text_key.to_string(),
                                        )
                                        .into(),
                                    )
                                }
                                _ => DIFValueContainer::from_value_container(
                                    &ValueContainer::from(k),
                                    memory,
                                ),
                            },
                            DIFValueContainer::from_value_container(v, memory),
                        )
                    })
                    .collect(),
            ),
        };

        DIFValue {
            value: dif_core_value,
            r#type: get_type_if_non_default(&value.actual_type, memory),
        }
    }
}

/// Returns the type if it is not the default type for the value, None otherwise
/// We treat the following types as default:
/// - boolean
/// - text
/// - null
/// - decimal (f64)
/// - List
/// - Map
fn get_type_if_non_default(
    type_container: &TypeContainer,
    memory: &RefCell<Memory>,
) -> Option<DIFTypeContainer> {
    match type_container {
        TypeContainer::TypeReference(inner) => {
            if let Some(Ok(address)) = inner
                .borrow()
                .pointer_address
                .as_ref()
                .map(CoreLibPointerId::try_from)
                && matches!(
                    address,
                    CoreLibPointerId::Decimal(Some(DecimalTypeVariant::F64))
                        | CoreLibPointerId::Boolean
                        | CoreLibPointerId::Text
                        | CoreLibPointerId::List
                        | CoreLibPointerId::Map
                        | CoreLibPointerId::Null
                )
            {
                None
            } else {
                Some(DIFTypeContainer::from_type_container(
                    type_container,
                    memory,
                ))
            }
        }
        _ => Some(DIFTypeContainer::from_type_container(
            type_container,
            memory,
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::dif::DIFConvertible;
    use crate::runtime::memory::Memory;
    use crate::values::core_values::endpoint::Endpoint;
    use crate::values::core_values::map::Map;
    use crate::values::value_container::ValueContainer;
    use crate::{
        dif::{r#type::DIFTypeContainer, value::DIFValue},
        libs::core::CoreLibPointerId,
        values::core_values::integer::typed_integer::IntegerTypeVariant,
    };
    use datex_core::values::value::Value;
    use std::cell::RefCell;

    fn get_mock_memory() -> RefCell<Memory> {
        RefCell::new(Memory::new(Endpoint::default()))
    }

    #[test]
    fn default_type() {
        let memory = get_mock_memory();
        let dif = DIFValue::from_value(&Value::from(true), &memory);
        assert!(dif.r#type.is_none());

        let dif = DIFValue::from_value(&Value::from("hello"), &memory);
        assert!(dif.r#type.is_none());

        let dif = DIFValue::from_value(&Value::null(), &memory);
        assert!(dif.r#type.is_none());

        let dif = DIFValue::from_value(&Value::from(3.5f64), &memory);
        assert!(dif.r#type.is_none());

        let dif = DIFValue::from_value(
            &Value::from(vec![Value::from(1), Value::from(2), Value::from(3)]),
            &memory,
        );
        assert!(dif.r#type.is_none());

        let dif = DIFValue::from_value(
            &Value::from(Map::from(vec![
                ("a".to_string(), ValueContainer::from(1)),
                ("b".to_string(), ValueContainer::from(2)),
            ])),
            &memory,
        );
        assert!(dif.r#type.is_none());
    }

    #[test]
    fn non_default_type() {
        let memory = get_mock_memory();
        let dif = DIFValue::from_value(&Value::from(123u16), &memory);
        assert!(dif.r#type.is_some());
        if let DIFTypeContainer::Reference(reference) = dif.r#type.unwrap() {
            assert_eq!(
                reference,
                CoreLibPointerId::Integer(Some(IntegerTypeVariant::U16)).into()
            );
        } else {
            panic!("Expected reference type");
        }

        let dif = DIFValue::from_value(&Value::from(123i64), &memory);
        assert!(dif.r#type.is_some());
        if let DIFTypeContainer::Reference(reference) = dif.r#type.unwrap() {
            assert_eq!(
                reference,
                CoreLibPointerId::Integer(Some(IntegerTypeVariant::I64)).into()
            );
        } else {
            panic!("Expected reference type");
        }
    }

    #[test]
    fn serde_dif_value() {
        let memory = get_mock_memory();
        let dif = DIFValue::from_value(&Value::from("Hello, world!"), &memory);
        let serialized = dif.as_json();
        let deserialized = DIFValue::from_json(&serialized);
        assert_eq!(dif, deserialized);
    }
}
