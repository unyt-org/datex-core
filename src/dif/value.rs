use crate::dif::r#type::DIFType;
use crate::dif::{
    core_value::DIFRepresentationValue, r#type::DIFTypeContainer,
};
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use datex_core::values::core_value::CoreValue;
use serde::{Deserialize, Serialize};

/// Represents a value in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFValue {
    pub value: DIFRepresentationValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<DIFTypeContainer>,
}

impl DIFValue {
    pub fn new(
        value: DIFRepresentationValue,
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

impl From<DIFRepresentationValue> for DIFValue {
    fn from(value: DIFRepresentationValue) -> Self {
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

impl From<&ValueContainer> for DIFValue {
    fn from(value: &ValueContainer) -> Self {
        let val_rc = value.to_value();
        let val = val_rc.borrow();
        let core_value = &val.inner;

        let dif_core_value = match core_value {
            CoreValue::Type(ty) => todo!("Type value not supported in DIF"),
            CoreValue::Null => DIFRepresentationValue::Null,
            CoreValue::Boolean(bool) => DIFRepresentationValue::Boolean(bool.0),
            CoreValue::Integer(integer) => {
                // TODO: optimize this and pass as integer if in range
                DIFRepresentationValue::String(integer.to_string())
            }
            CoreValue::TypedInteger(integer) => {
                // Some(DIFCoreValue::Number(integer.as_i64().unwrap() as f64))
                match integer {
                    TypedInteger::I8(i) => {
                        DIFRepresentationValue::Number(*i as f64)
                    }
                    TypedInteger::U8(u) => {
                        DIFRepresentationValue::Number(*u as f64)
                    }
                    TypedInteger::I16(i) => {
                        DIFRepresentationValue::Number(*i as f64)
                    }
                    TypedInteger::U16(u) => {
                        DIFRepresentationValue::Number(*u as f64)
                    }
                    TypedInteger::I32(i) => {
                        DIFRepresentationValue::Number(*i as f64)
                    }
                    TypedInteger::U32(u) => {
                        DIFRepresentationValue::Number(*u as f64)
                    }
                    // i64 and above are serialized as strings in DIF
                    TypedInteger::I64(i) => {
                        DIFRepresentationValue::String(i.to_string())
                    }
                    TypedInteger::U64(u) => {
                        DIFRepresentationValue::String(u.to_string())
                    }
                    TypedInteger::I128(i) => {
                        DIFRepresentationValue::String(i.to_string())
                    }
                    TypedInteger::U128(u) => {
                        DIFRepresentationValue::String(u.to_string())
                    }
                    TypedInteger::Big(i) => {
                        DIFRepresentationValue::String(i.to_string())
                    }
                }
            }
            CoreValue::Decimal(decimal) => {
                // TODO: optimize this and pass as decimal if in range
                DIFRepresentationValue::String(decimal.to_string())
            }
            CoreValue::TypedDecimal(decimal) => match decimal {
                TypedDecimal::F32(f) => {
                    DIFRepresentationValue::Number(f.0 as f64)
                }
                TypedDecimal::F64(f) => DIFRepresentationValue::Number(f.0),
                TypedDecimal::Decimal(bd) => {
                    DIFRepresentationValue::String(bd.to_string())
                }
            },
            CoreValue::Text(text) => {
                DIFRepresentationValue::String(text.0.clone())
            }
            CoreValue::Endpoint(endpoint) => {
                DIFRepresentationValue::String(endpoint.to_string())
            }
            CoreValue::Struct(structure) => DIFRepresentationValue::Object(
                structure
                    .iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            DIFValueContainer::from(DIFValue::from(value)),
                        )
                    })
                    .collect(),
            ),
            _ => unimplemented!(
                "Conversion for core value {:?} not implemented yet",
                core_value
            ),
            // CoreValue::List(list) => Some(DIFCoreValue::Array(
            //     list.into_iter().map(|v| v.into()).collect(),
            // )),
            // CoreValue::Array(arr) => Some(DIFCoreValue::Array(
            //     arr.into_iter().map(|v| v.into()).collect(),
            // )),
            // CoreValue::Map(map) => Some(DIFCoreValue::Map(
            //     map.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
            // )),
        };

        DIFValue {
            value: dif_core_value,
            r#type: Some(value.actual_type().into()),
        }
    }
}
