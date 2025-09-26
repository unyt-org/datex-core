use crate::references::reference::ReferenceMutability;
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::text::Text;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use datex_core::values::core_value::CoreValue;
use datex_core::values::core_values::integer::integer::Integer;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DIFTypeContainer {
    Value(DIFType),
    Reference(PointerAddress),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability: Option<ReferenceMutability>,
    pub type_definition: TypeDefinition,
}

impl From<TypeContainer> for DIFTypeContainer {
    fn from(type_container: TypeContainer) -> Self {
        match type_container {
            TypeContainer::Type(ty) => DIFTypeContainer::Value(DIFType {
                name: None,
                mutability: ty.reference_mutability,
                type_definition: ty.type_definition,
            }),
            TypeContainer::TypeReference(type_ref) => {
                DIFTypeContainer::Reference(
                    type_ref.borrow().pointer_address.clone().unwrap(),
                )
            }
        }
    }
}
impl From<TypeContainer> for DIFType {
    fn from(type_container: TypeContainer) -> Self {
        match type_container {
            TypeContainer::Type(ty) => DIFType {
                name: None,
                mutability: ty.reference_mutability,
                type_definition: ty.type_definition,
            },
            TypeContainer::TypeReference(type_ref) => {
                let type_ref = type_ref.borrow().collapse_reference_chain();
                let actual_type = type_ref.type_value;
                let name = type_ref
                    .nominal_type_declaration
                    .as_ref()
                    .map(|decl| decl.name.clone());
                DIFType {
                    name,
                    mutability: actual_type.reference_mutability,
                    type_definition: actual_type.type_definition,
                }
            }
        }
    }
}
