use crate::types::definition::TypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::pointer::PointerAddress;
use crate::{dif::value::DIFValue, references::reference::ReferenceMutability};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "def")]
pub enum DIFTypeDefinition {
    // {x: integer, y: text}
    #[serde(rename = "structural")]
    Structural(Box<DIFValue>),

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

impl From<TypeDefinition> for DIFTypeDefinition {
    fn from(type_def: TypeDefinition) -> Self {
        match type_def {
            TypeDefinition::Structural(struct_def) => {
                DIFTypeDefinition::Structural(Box::new(DIFValue {
                    value: struct_def.into(),
                    r#type: None,
                }))
            }
            TypeDefinition::Reference(type_ref) => {
                DIFTypeDefinition::Reference(
                    type_ref.borrow().pointer_address.clone().unwrap(),
                )
            }
            TypeDefinition::Intersection(types) => {
                DIFTypeDefinition::Intersection(
                    types.into_iter().map(DIFTypeContainer::from).collect(),
                )
            }
            TypeDefinition::Union(types) => DIFTypeDefinition::Union(
                types.into_iter().map(DIFTypeContainer::from).collect(),
            ),
            TypeDefinition::Unit => DIFTypeDefinition::Unit,
            TypeDefinition::Function {
                parameters,
                return_type,
            } => DIFTypeDefinition::Function {
                parameters: parameters
                    .into_iter()
                    .map(|(name, ty)| (name, ty.into()))
                    .collect(),
                return_type: Box::new((*return_type).into()),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DIFTypeContainer {
    Type(DIFType),
    Reference(PointerAddress),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mut")]
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

impl From<TypeContainer> for DIFTypeContainer {
    fn from(type_container: TypeContainer) -> Self {
        match type_container {
            TypeContainer::Type(ty) => DIFTypeContainer::Type(DIFType {
                name: None,
                mutability: ty.reference_mutability,
                type_definition: ty.type_definition.into(),
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
                type_definition: ty.type_definition.into(),
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
                    type_definition: actual_type.type_definition.into(),
                }
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
        dif::core_value::DIFRepresentationValue,
        types::structural_type_definition::StructuralTypeDefinition,
        values::core_values::r#type::Type,
    };

    use super::*;
    #[test]
    fn dif_type_serialization() {
        let dif_type = DIFType {
            name: Some("ExampleType".to_string()),
            mutability: Some(ReferenceMutability::Mutable),
            type_definition: TypeDefinition::Structural(
                StructuralTypeDefinition::Null,
            )
            .into(),
        };
        let serialized = serde_json::to_string(&dif_type).unwrap();
        println!("Serialized DIFType: {}", serialized);
        let deserialized: DIFType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn dif_type_serialization_2() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFValue {
                    value: DIFRepresentationValue::Object(vec![
                        (
                            "field1".to_string(),
                            DIFValue::from(DIFRepresentationValue::Null)
                                .as_container(),
                        ),
                        (
                            "field2".to_string(),
                            DIFValue::from(DIFRepresentationValue::Number(
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
}
