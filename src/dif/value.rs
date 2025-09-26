use crate::dif::r#type::DIFType;
use crate::dif::{
    core_value::DIFRepresentationValue, r#type::DIFTypeContainer,
};
use crate::libs::core::{CoreLibPointerId, get_core_lib_type_reference};
use crate::types::type_container::TypeContainer;
use crate::values::core_values::decimal::typed_decimal::{
    DecimalTypeVariant, TypedDecimal,
};
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use datex_core::values::core_value::CoreValue;
use serde::{Deserialize, Serialize};

/// Represents a value in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFValue {
    pub value: DIFRepresentationValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<DIFTypeContainer>,

    // used for references
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_type: Option<DIFTypeContainer>,
}

impl DIFValue {
    pub fn new(
        value: DIFRepresentationValue,
        r#type: Option<impl Into<DIFTypeContainer>>,
    ) -> Self {
        DIFValue {
            value,
            allowed_type: None,
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
            allowed_type: None,
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
    fn from(value_container: &ValueContainer) -> Self {
        let val_rc = value_container.to_value();
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
            CoreValue::List(list) => DIFRepresentationValue::Array(
                list.iter()
                    .map(|v| DIFValueContainer::from(DIFValue::from(v)))
                    .collect(),
            ),
            CoreValue::Array(arr) => DIFRepresentationValue::Array(
                arr.iter()
                    .map(|v| DIFValueContainer::from(DIFValue::from(v)))
                    .collect(),
            ),
            CoreValue::Map(map) => DIFRepresentationValue::Map(
                map.iter()
                    .map(|(k, v)| {
                        (
                            DIFValueContainer::from(DIFValue::from(k)),
                            DIFValueContainer::from(DIFValue::from(v)),
                        )
                    })
                    .collect(),
            ),
        };

        DIFValue {
            value: dif_core_value,
            allowed_type: get_allowed_type(value_container),
            r#type: get_type_if_non_default(value_container.actual_type()),
        }
    }
}

impl From<ValueContainer> for DIFValue {
    fn from(value: ValueContainer) -> Self {
        DIFValue::from(&value)
    }
}

/// Returns the allowed type for references, None for other value containers
fn get_allowed_type(
    value_container: &ValueContainer,
) -> Option<DIFTypeContainer> {
    match &value_container {
        ValueContainer::Reference(reference) => {
            let allowed_type = reference.allowed_type();
            Some(allowed_type.into())
        }
        _ => None,
    }
}

/// Returns the type if it is not the default type for the value, None otherwise
/// We treet the following types as default:
/// - Boolean
/// - Text
/// - Null
/// - Decimal (f64)
/// - List
/// - Map
fn get_type_if_non_default(r#type: TypeContainer) -> Option<DIFTypeContainer> {
    match &r#type {
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
                        | CoreLibPointerId::Null // | CoreLibPointerId::Struct
                )
            {
                None
            } else {
                Some(DIFTypeContainer::from(r#type))
            }
        }
        _ => Some(DIFTypeContainer::from(r#type)),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dif::{r#type::DIFTypeContainer, value::DIFValue},
        libs::core::CoreLibPointerId,
        values::{
            core_values::integer::typed_integer::IntegerTypeVariant,
            value_container::ValueContainer,
        },
    };

    #[test]
    fn default_type() {
        let dif = DIFValue::from(ValueContainer::from(true));
        assert!(dif.r#type.is_none());

        let dif = DIFValue::from(ValueContainer::from("hello"));
        assert!(dif.r#type.is_none());
    }

    #[test]
    fn non_default_type() {
        let dif = DIFValue::from(ValueContainer::from(123u16));
        assert!(dif.r#type.is_some());
        if let DIFTypeContainer::Reference(reference) = dif.r#type.unwrap() {
            assert_eq!(
                reference,
                CoreLibPointerId::Integer(Some(IntegerTypeVariant::U16)).into()
            );
        } else {
            panic!("Expected reference type");
        }
    }
}
