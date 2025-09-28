use crate::dif::{
    dif_representation::DIFRepresentationValue, r#type::DIFTypeContainer,
};
use crate::libs::core::CoreLibPointerId;
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
use datex_core::runtime::memory::Memory;

#[derive(Debug)]
pub struct DIFReferenceNotFoundError;

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
    /// Converts the DIFValue into a Value, resolving references using the provided memory.
    /// Returns an error if a reference cannot be resolved.
    pub fn to_value(self, memory: &Memory) -> Result<Value, DIFReferenceNotFoundError> {
        Ok(
            if let Some(r#type) = &self.r#type {
                self.value.to_value_with_type(r#type, memory)?
            }
            else {
                self.value.to_default_value(memory)?
            }
        )
    }
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

impl DIFValueContainer {
    /// Converts the DIFValueContainer into a ValueContainer, resolving references using the provided memory.
    /// Returns an error if a reference cannot be resolved.
    pub fn to_value_container(self, memory: &Memory) -> Result<ValueContainer, DIFReferenceNotFoundError> {
        Ok(
            match self {
                DIFValueContainer::Reference(address) => ValueContainer::Reference(
                    memory.get_reference(&address)
                        .ok_or(DIFReferenceNotFoundError)?
                        .clone(),
                ),
                DIFValueContainer::Value(v) => ValueContainer::Value(v.to_value(memory)?)
            }
        )
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

#[derive(Debug)]
pub enum TryIntoDIFError {
    /// If the ValueContainer contains a Reference that has no pointer address assigned
    MissingReferenceAddress,
}

impl TryFrom<&ValueContainer> for DIFValueContainer {
    type Error = TryIntoDIFError;
    fn try_from(value_container: &ValueContainer) -> Result<Self, Self::Error> {
        match value_container {
            ValueContainer::Reference(reference) => {
                let address = reference
                    .pointer_address()
                    .ok_or(TryIntoDIFError::MissingReferenceAddress)?;
                Ok(DIFValueContainer::Reference(address.clone()))
            }
            ValueContainer::Value(value) => {
                Ok(DIFValueContainer::Value(DIFValue::try_from(value)?))
            }
        }
    }
}

// FIXME do we really need a TryFrom here?
impl TryFrom<&Value> for DIFValue {
    type Error = TryIntoDIFError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let core_value = &value.inner;

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
                        DIFValueContainer::try_from(value)
                            .map(|v| (key.clone(), v))
                    })
                    .collect::<Result<Vec<(String, DIFValueContainer)>, _>>()?,
            ),
            CoreValue::List(list) => DIFRepresentationValue::Array(
                list.iter()
                    .map(DIFValueContainer::try_from)
                    .collect::<Result<Vec<DIFValueContainer>, _>>()?,
            ),
            CoreValue::Array(arr) => DIFRepresentationValue::Array(
                arr.iter()
                    .map(DIFValueContainer::try_from)
                    .collect::<Result<Vec<DIFValueContainer>, _>>()?,
            ),
            CoreValue::Map(map) => {
                DIFRepresentationValue::Map(
                    map.iter()
                        .map(|(k, v)| {
                            DIFValueContainer::try_from(k).and_then(|key| {
                                DIFValueContainer::try_from(v)
                                    .map(|val| (key, val))
                            })
                        })
                        .collect::<Result<
                            Vec<(DIFValueContainer, DIFValueContainer)>,
                            _,
                        >>()?,
                )
            }
        };

        Ok(DIFValue {
            value: dif_core_value,
            allowed_type: None,
            r#type: get_type_if_non_default(&value.actual_type),
        })
    }
}

// TODO: handle allowed type for references (must be set after try_from<Value> for references)
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
fn get_type_if_non_default(r#type: &TypeContainer) -> Option<DIFTypeContainer> {
    match r#type {
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
        values::core_values::integer::typed_integer::IntegerTypeVariant,
    };
    use datex_core::values::value::Value;

    #[test]
    fn default_type() {
        let dif = DIFValue::try_from(&Value::from(true)).unwrap();
        assert!(dif.r#type.is_none());

        let dif = DIFValue::try_from(&Value::from("hello")).unwrap();
        assert!(dif.r#type.is_none());
    }

    #[test]
    fn non_default_type() {
        let dif = DIFValue::try_from(&Value::from(123u16)).unwrap();
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

    #[test]
    fn serde_dif_value() {
        let dif = DIFValue::try_from(&Value::from("Hello, world!")).unwrap();
        let serialized = serde_json::to_string(&dif).unwrap();
        println!("Serialized DIFValue: {}", serialized);
        let deserialized: DIFValue = serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif, deserialized);
    }
    // #[test]
    // fn allowed_type() {
    //     let dif = DIFValue::from(ValueContainer::Reference(
    //         CoreLibPointerId::Integer(Some(IntegerTypeVariant::I32)).into(),
    //         Some(CoreLibPointerId::Number.into()),
    //     ));
    //     assert!(dif.allowed_type.is_some());
    //     if let DIFTypeContainer::Reference(reference) =
    //         dif.allowed_type.unwrap()
    //     {
    //         assert_eq!(reference, CoreLibPointerId::Number.into());
    //     } else {
    //         panic!("Expected reference type");
    //     }
    // }
}
