use std::cell::RefCell;
use crate::types::definition::TypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::pointer::PointerAddress;
use crate::{dif::value::DIFValue, references::reference::ReferenceMutability};
use serde::{Deserialize, Serialize};
use crate::dif::dif_representation::DIFTypeRepresentation;
use crate::references::reference::Reference;
use crate::runtime::memory::Memory;
use crate::types::structural_type_definition::StructuralTypeDefinition;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "def")]
pub enum DIFTypeDefinition {
    // {x: integer, y: text}
    #[serde(rename = "structural")]
    Structural(Box<DIFStructuralTypeDefinition>),

    #[serde(rename = "reference")]
    Reference(PointerAddress),

    // e.g. A & B & C
    #[serde(rename = "intersection")]
    Intersection(Vec<DIFTypeContainer>),

    // e.g. A | B | C
    #[serde(rename = "union")]
    Union(Vec<DIFTypeContainer>),

    // ()
    #[serde(rename = "unit")]
    Unit,

    #[serde(rename = "function")]
    Function {
        parameters: Vec<(String, DIFTypeContainer)>,
        return_type: Box<DIFTypeContainer>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFStructuralTypeDefinition {
    pub value: DIFTypeRepresentation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<DIFTypeContainer>,
}

impl DIFStructuralTypeDefinition {
    fn from_structural_definition(
        struct_def: &StructuralTypeDefinition,
        memory: &RefCell<Memory>,
    ) -> Self {
        let value = DIFTypeRepresentation::from_structural_type_definition(struct_def, memory);
        let type_def = PointerAddress::from(struct_def.get_core_lib_type_pointer_id());
        DIFStructuralTypeDefinition {
            value,
            r#type: Some(DIFTypeContainer::Reference(type_def)),
        }
    }
}


impl DIFTypeDefinition {
    fn from_type_definition(type_def: &TypeDefinition, memory: &RefCell<Memory>) -> Self {
        match type_def {
            TypeDefinition::Structural(struct_def) => {
                DIFTypeDefinition::Structural(Box::new(DIFStructuralTypeDefinition::from_structural_definition(struct_def, memory)))
            }
            TypeDefinition::Reference(type_ref) => {
                DIFTypeDefinition::Reference(
                    type_ref.borrow().pointer_address.clone().unwrap(),
                )
            }
            TypeDefinition::Intersection(types) => {
                DIFTypeDefinition::Intersection(
                    types.iter().map(|t| DIFTypeContainer::from_type_container(t, memory)).collect(),
                )
            }
            TypeDefinition::Union(types) => DIFTypeDefinition::Union(
                types.iter().map(|t| DIFTypeContainer::from_type_container(t, memory)).collect(),
            ),
            TypeDefinition::Unit => DIFTypeDefinition::Unit,
            TypeDefinition::Function {
                parameters,
                return_type,
            } => DIFTypeDefinition::Function {
                parameters: parameters
                    .iter()
                    .map(|(name, ty)| (name.clone(), DIFTypeContainer::from_type_container(ty, memory)))
                    .collect(),
                return_type: Box::new(DIFTypeContainer::from_type_container(return_type, memory)),
            },
        }
    }
}

impl From<DIFTypeContainer> for TypeContainer {
    fn from(dif_type_container: DIFTypeContainer) -> Self {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DIFTypeContainer {
    Type(DIFType),
    Reference(PointerAddress),
}

impl DIFTypeContainer {
    pub fn none() -> Option<Self> {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mut")]
    #[serde(default)]
    #[serde(with = "mutability_as_int")]
    pub mutability: Option<ReferenceMutability>,

    // untagged
    #[serde(flatten)]
    pub type_definition: DIFTypeDefinition,
}

impl DIFType {
    pub fn as_container(self) -> DIFTypeContainer {
        DIFTypeContainer::Type(self)
    }
}

impl From<DIFTypeRepresentation> for DIFType {
    fn from(value: DIFTypeRepresentation) -> Self {
        DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value,
                    r#type: None,
                },
            )),
        }
    }
}


impl DIFTypeContainer {
    pub fn from_type_container(type_container: &TypeContainer, memory: &RefCell<Memory>) -> Self {
        match type_container {
            TypeContainer::Type(ty) => DIFTypeContainer::Type(DIFType {
                name: None,
                mutability: ty.reference_mutability.clone(),
                type_definition: DIFTypeDefinition::from_type_definition(&ty.type_definition, memory),
            }),
            TypeContainer::TypeReference(type_ref) => {
                let type_ref_borrow = type_ref.borrow();
                let address = if let Some(ref address) = type_ref_borrow.pointer_address {
                    address
                } else {
                    &memory.borrow_mut().register_reference(&Reference::TypeReference(type_ref.clone()))
                };
                DIFTypeContainer::Reference(address.clone())
            }
        }
    }
}

mod mutability_as_int {
    use super::ReferenceMutability;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        value: &Option<ReferenceMutability>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(ReferenceMutability::Mutable) => serializer.serialize_u8(0),
            Some(ReferenceMutability::Immutable) => serializer.serialize_u8(1),
            Some(ReferenceMutability::Final) => serializer.serialize_u8(2),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<ReferenceMutability>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<u8>::deserialize(deserializer)?;
        Ok(match opt {
            Some(0) => Some(ReferenceMutability::Mutable),
            Some(1) => Some(ReferenceMutability::Immutable),
            Some(2) => Some(ReferenceMutability::Final),
            Some(x) => {
                return Err(D::Error::custom(format!(
                    "invalid mutability code: {}",
                    x
                )));
            }
            None => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dif::dif_representation::DIFValueRepresentation,
        types::structural_type_definition::StructuralTypeDefinition,
    };

    use super::*;
    #[test]
    fn dif_type_serialization() {
        let dif_type = DIFType {
            name: Some("ExampleType".to_string()),
            mutability: Some(ReferenceMutability::Mutable),
            type_definition: DIFTypeDefinition::Unit,
        };
        let serialized = serde_json::to_string(&dif_type).unwrap();
        println!("Serialized DIFType: {}", serialized);
        let deserialized: DIFType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn r#struct() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value: DIFTypeRepresentation::Object(vec![
                        (
                            "field1".to_string(),
                            DIFType::from(DIFTypeRepresentation::Null)
                                .as_container(),
                        ),
                        (
                            "field2".to_string(),
                            DIFType::from(DIFTypeRepresentation::Number(
                                42.0,
                            ))
                            .as_container(),
                        ),
                    ]),
                    r#type: None,
                },
            )),
        };
        let serialized = serde_json::to_string(&dif_type).unwrap();
        println!("Serialized DIFType: {}", serialized);
        let deserialized: DIFType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn map() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value: DIFTypeRepresentation::Map(vec![
                        (
                            DIFType::from(DIFTypeRepresentation::String(
                                "key1".to_string(),
                            ))
                            .as_container(),
                            DIFType::from(DIFTypeRepresentation::Number(
                                42.0,
                            ))
                            .as_container(),
                        ),
                        (
                            DIFType::from(DIFTypeRepresentation::Number(1.0))
                                .as_container(),
                            DIFType::from(DIFTypeRepresentation::Number(3.5))
                                .as_container(),
                        ),
                    ]),
                    r#type: None,
                },
            )),
        };
        let serialized = serde_json::to_string_pretty(&dif_type).unwrap();
        println!("Serialized DIFType: {}", serialized);
        let deserialized: DIFType = serde_json::from_str(&serialized).unwrap();
        println!("Deserialized DIFType: {:#?}", deserialized);
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn array() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value: DIFTypeRepresentation::Array(vec![
                        DIFType::from(DIFTypeRepresentation::Number(1.0))
                            .as_container(),
                        DIFType::from(DIFTypeRepresentation::Number(2.0))
                            .as_container(),
                        DIFType::from(DIFTypeRepresentation::Number(3.0))
                            .as_container(),
                    ]),
                    r#type: None,
                },
            )),
        };
        let serialized = serde_json::to_string_pretty(&dif_type).unwrap();
        println!("Serialized DIFType: {}", serialized);
        let deserialized: DIFType = serde_json::from_str(&serialized).unwrap();
        println!("Deserialized DIFType: {:#?}", deserialized);
        assert_eq!(dif_type, deserialized);
    }
}
