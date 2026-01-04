use crate::dif::DIFConvertible;
use crate::dif::representation::DIFTypeRepresentation;
use crate::references::reference::ReferenceMutability;
use crate::runtime::memory::Memory;
use crate::stdlib::boxed::Box;
use crate::stdlib::format;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use core::cell::RefCell;
use core::prelude::rust_2024::*;
use core::prelude::rust_2024::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::de::IntoDeserializer;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum DIFTypeDefinition {
    // {x: integer, y: text}
    Structural(Box<DIFStructuralTypeDefinition>),

    Reference(PointerAddress),
    Type(Box<DIFType>),

    // e.g. A & B & C
    Intersection(Vec<DIFType>),

    // e.g. A | B | C
    Union(Vec<DIFType>),

    ImplType(Box<DIFType>, Vec<PointerAddress>),

    // ()
    Unit,

    Never,

    Unknown,

    Callable {
        parameters: Vec<(Option<String>, DIFType)>,
        rest_parameter: Option<(Option<String>, Box<DIFType>)>,
        return_type: Option<Box<DIFType>>,
        yeet_type: Option<Box<DIFType>>,
    },
}

#[repr(u8)]
#[derive(Debug, TryFromPrimitive, IntoPrimitive)]
pub enum DIFTypeDefinitionKind {
    Structural = 1,
    Reference = 2,
    Type = 3,
    Intersection = 4,
    Union = 5,
    ImplType = 6,
    Unit = 7,
    Never = 8,
    Unknown = 9,
    Callable = 10,
}

impl From<&DIFTypeDefinition> for DIFTypeDefinitionKind {
    fn from(value: &DIFTypeDefinition) -> Self {
        match value {
            DIFTypeDefinition::Structural(_) => {
                DIFTypeDefinitionKind::Structural
            }
            DIFTypeDefinition::Reference(_) => DIFTypeDefinitionKind::Reference,
            DIFTypeDefinition::Type(_) => DIFTypeDefinitionKind::Type,
            DIFTypeDefinition::Intersection(_) => {
                DIFTypeDefinitionKind::Intersection
            }
            DIFTypeDefinition::Union(_) => DIFTypeDefinitionKind::Union,
            DIFTypeDefinition::ImplType(_, _) => {
                DIFTypeDefinitionKind::ImplType
            }
            DIFTypeDefinition::Unit => DIFTypeDefinitionKind::Unit,
            DIFTypeDefinition::Never => DIFTypeDefinitionKind::Never,
            DIFTypeDefinition::Unknown => DIFTypeDefinitionKind::Unknown,
            DIFTypeDefinition::Callable { .. } => {
                DIFTypeDefinitionKind::Callable
            }
        }
    }
}

// custom serialize impl, convert to tagged enum, with integers for kind
impl Serialize for DIFTypeDefinition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // if DIFTypeDefinition::Reference, just serialize the PointerAddress directly as string
        if let DIFTypeDefinition::Reference(type_ref) = self {
            return type_ref.serialize(serializer);
        }

        let kind = DIFTypeDefinitionKind::from(self);
        let len = match self {
            DIFTypeDefinition::Unit => 1,
            DIFTypeDefinition::Never => 1,
            DIFTypeDefinition::Unknown => 1,
            _ => 2,
        };
        let mut state =
            serializer.serialize_struct("DIFTypeDefinition", len)?;
        state.serialize_field("kind", &(kind as u8))?;
        match self {
            DIFTypeDefinition::Structural(def) => {
                state.serialize_field("def", def)?;
            }
            DIFTypeDefinition::Type(ty) => {
                state.serialize_field("def", ty)?;
            }
            DIFTypeDefinition::Intersection(types) => {
                state.serialize_field("def", types)?;
            }
            DIFTypeDefinition::Union(types) => {
                state.serialize_field("def", types)?;
            }
            DIFTypeDefinition::ImplType(ty, impls) => {
                state.serialize_field("def", &(ty, impls))?;
            }
            DIFTypeDefinition::Unit => {
                // no def field
            }
            DIFTypeDefinition::Never => {
                // no def field
            }
            DIFTypeDefinition::Unknown => {
                // no def field
            }
            DIFTypeDefinition::Callable {
                parameters,
                rest_parameter,
                return_type,
                yeet_type,
            } => {
                state.serialize_field(
                    "def",
                    &(parameters, rest_parameter, return_type, yeet_type),
                )?;
            }
            DIFTypeDefinition::Reference(_) => {
                // already handled above
                unreachable!();
            }
        }
        state.end()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum DIFTypeDefinitionData {
    Structural(DIFStructuralTypeDefinition),
    Reference(PointerAddress),
    SingleType(DIFType),
    TypeVec(Vec<DIFType>),
    ImplType((DIFType, Vec<PointerAddress>)),
    Callable(
        (
            Vec<(Option<String>, DIFType)>,
            Option<(Option<String>, Box<DIFType>)>,
            Option<Box<DIFType>>,
            Option<Box<DIFType>>,
        ),
    ),
}

impl<'de> Deserialize<'de> for DIFTypeDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        struct DIFTypeDefinitionVisitor;

        impl<'de> Visitor<'de> for DIFTypeDefinitionVisitor {
            type Value = DIFTypeDefinition;

            fn expecting(
                &self,
                formatter: &mut core::fmt::Formatter,
            ) -> core::fmt::Result {
                formatter.write_str("struct DIFTypeDefinition")
            }

            // reference from PointerAddress string representation
            fn visit_str<E>(self, v: &str) -> Result<DIFTypeDefinition, E>
            where
                E: de::Error,
            {
                let type_ref =
                    PointerAddress::deserialize(v.into_deserializer())?;
                Ok(DIFTypeDefinition::Reference(type_ref))
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> Result<DIFTypeDefinition, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut kind: Option<u8> = None;
                let mut def: Option<DIFTypeDefinitionData> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" => kind = Some(map.next_value()?),
                        "def" => def = Some(map.next_value()?),
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["kind", "def"],
                            ));
                        }
                    }
                }

                let kind =
                    kind.ok_or_else(|| de::Error::missing_field("kind"))?;
                let kind =
                    DIFTypeDefinitionKind::try_from(kind).map_err(|_| {
                        de::Error::custom(format!(
                            "Invalid kind value: {}",
                            kind
                        ))
                    })?;
                Ok(match kind {
                    DIFTypeDefinitionKind::Structural => {
                        let def =
                            def.ok_or_else(|| de::Error::missing_field("def"))?;
                        if let DIFTypeDefinitionData::Structural(struct_def) =
                            def
                        {
                            DIFTypeDefinition::Structural(Box::new(struct_def))
                        } else {
                            return Err(de::Error::custom(
                                "Expected StructuralTypeDefinition for kind Structural",
                            ));
                        }
                    }
                    DIFTypeDefinitionKind::Reference => {
                        let def =
                            def.ok_or_else(|| de::Error::missing_field("def"))?;
                        if let DIFTypeDefinitionData::Reference(type_ref) = def
                        {
                            DIFTypeDefinition::Reference(type_ref)
                        } else {
                            return Err(de::Error::custom(
                                "Expected PointerAddress for kind Reference",
                            ));
                        }
                    }
                    DIFTypeDefinitionKind::Type => {
                        let def =
                            def.ok_or_else(|| de::Error::missing_field("def"))?;
                        if let DIFTypeDefinitionData::SingleType(ty) = def {
                            DIFTypeDefinition::Type(Box::new(ty))
                        } else {
                            return Err(de::Error::custom(
                                "Expected DIFType for kind Type",
                            ));
                        }
                    }
                    DIFTypeDefinitionKind::Intersection => {
                        let def =
                            def.ok_or_else(|| de::Error::missing_field("def"))?;
                        if let DIFTypeDefinitionData::TypeVec(types) = def {
                            DIFTypeDefinition::Intersection(types)
                        } else {
                            return Err(de::Error::custom(
                                "Expected Vec<DIFType> for kind Intersection",
                            ));
                        }
                    }
                    DIFTypeDefinitionKind::Union => {
                        let def =
                            def.ok_or_else(|| de::Error::missing_field("def"))?;
                        if let DIFTypeDefinitionData::TypeVec(types) = def {
                            DIFTypeDefinition::Union(types)
                        } else {
                            return Err(de::Error::custom(
                                "Expected Vec<DIFType> for kind Union",
                            ));
                        }
                    }
                    DIFTypeDefinitionKind::ImplType => {
                        let def =
                            def.ok_or_else(|| de::Error::missing_field("def"))?;
                        if let DIFTypeDefinitionData::ImplType((ty, impls)) =
                            def
                        {
                            DIFTypeDefinition::ImplType(Box::new(ty), impls)
                        } else {
                            return Err(de::Error::custom(
                                "Expected (DIFType, Vec<PointerAddress>) for kind ImplType",
                            ));
                        }
                    }
                    DIFTypeDefinitionKind::Unit => DIFTypeDefinition::Unit,
                    DIFTypeDefinitionKind::Never => DIFTypeDefinition::Never,
                    DIFTypeDefinitionKind::Unknown => {
                        DIFTypeDefinition::Unknown
                    }
                    DIFTypeDefinitionKind::Callable => {
                        let def =
                            def.ok_or_else(|| de::Error::missing_field("def"))?;
                        if let DIFTypeDefinitionData::Callable((
                            parameters,
                            rest_parameter,
                            return_type,
                            yeet_type,
                        )) = def
                        {
                            DIFTypeDefinition::Callable {
                                parameters,
                                rest_parameter,
                                return_type,
                                yeet_type,
                            }
                        } else {
                            return Err(de::Error::custom(
                                "Expected (Vec<(String, DIFType)>, Box<DIFType>) for kind Function",
                            ));
                        }
                    }
                })
            }
        }

        deserializer.deserialize_any(DIFTypeDefinitionVisitor)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFStructuralTypeDefinition {
    pub value: DIFTypeRepresentation,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub ty: Option<DIFType>,
}

impl DIFStructuralTypeDefinition {
    fn from_structural_definition(
        struct_def: &StructuralTypeDefinition,
        memory: &RefCell<Memory>,
    ) -> Self {
        let value = DIFTypeRepresentation::from_structural_type_definition(
            struct_def, memory,
        );
        let type_def =
            PointerAddress::from(struct_def.get_core_lib_type_pointer_id());
        DIFStructuralTypeDefinition {
            value,
            ty: Some(DIFType {
                type_definition: DIFTypeDefinition::Reference(type_def),
                mutability: None,
                name: None,
            }),
        }
    }
}

impl DIFTypeDefinition {
    pub fn from_type_definition(
        type_def: &TypeDefinition,
        memory: &RefCell<Memory>,
    ) -> Self {
        match type_def {
            TypeDefinition::Collection(collection_def) => {
                core::todo!("#387 handle collection type conversion");
            }
            TypeDefinition::Structural(struct_def) => {
                DIFTypeDefinition::Structural(Box::new(
                    DIFStructuralTypeDefinition::from_structural_definition(
                        struct_def, memory,
                    ),
                ))
            }
            TypeDefinition::Reference(type_ref) => {
                DIFTypeDefinition::Reference(
                    type_ref.borrow().pointer_address.clone().unwrap(),
                )
            }
            TypeDefinition::Type(type_val) => DIFTypeDefinition::Type(
                Box::new(DIFType::from_type(type_val.as_ref(), memory)),
            ),
            TypeDefinition::Intersection(types) => {
                DIFTypeDefinition::Intersection(
                    types
                        .iter()
                        .map(|t| DIFType::from_type(t, memory))
                        .collect(),
                )
            }
            TypeDefinition::Union(types) => DIFTypeDefinition::Union(
                types
                    .iter()
                    .map(|t| DIFType::from_type(t, memory))
                    .collect(),
            ),
            TypeDefinition::ImplType(ty, impls) => DIFTypeDefinition::ImplType(
                Box::new(DIFType::from_type(ty, memory)),
                impls.clone(),
            ),
            TypeDefinition::Unit => DIFTypeDefinition::Unit,
            TypeDefinition::Never => DIFTypeDefinition::Never,
            TypeDefinition::Unknown => DIFTypeDefinition::Unknown,
            TypeDefinition::Callable(callable) => DIFTypeDefinition::Callable {
                parameters: callable
                    .parameter_types
                    .iter()
                    .map(|(name, ty)| {
                        (name.clone(), DIFType::from_type(ty, memory))
                    })
                    .collect(),
                rest_parameter: callable.rest_parameter_type.as_ref().map(
                    |(name, ty)| {
                        (
                            name.clone(),
                            Box::new(DIFType::from_type(ty.as_ref(), memory)),
                        )
                    },
                ),
                yeet_type: callable.yeet_type.as_ref().map(|ty| {
                    Box::new(DIFType::from_type(ty.as_ref(), memory))
                }),
                return_type: callable.return_type.as_ref().map(|ty| {
                    Box::new(DIFType::from_type(ty.as_ref(), memory))
                }),
            },
        }
    }

    pub(crate) fn to_type_definition(
        &self,
        memory: &RefCell<Memory>,
    ) -> TypeDefinition {
        match self {
            DIFTypeDefinition::Intersection(types) => {
                TypeDefinition::Intersection(
                    types.iter().map(|t| t.to_type(memory)).collect(),
                )
            }
            DIFTypeDefinition::Union(types) => TypeDefinition::Union(
                types.iter().map(|t| t.to_type(memory)).collect(),
            ),
            DIFTypeDefinition::Reference(type_ref_addr) => {
                let type_ref = memory
                    .borrow_mut()
                    .get_type_reference(type_ref_addr)
                    .expect("Reference not found in memory")
                    .clone();
                TypeDefinition::Reference(type_ref)
            }
            DIFTypeDefinition::Type(dif_type) => {
                TypeDefinition::Type(Box::new(dif_type.to_type(memory)))
            }
            DIFTypeDefinition::ImplType(ty, impls) => TypeDefinition::ImplType(
                Box::new(ty.to_type(memory)),
                impls.clone(),
            ),
            DIFTypeDefinition::Unit => TypeDefinition::Unit,
            DIFTypeDefinition::Never => TypeDefinition::Never,
            DIFTypeDefinition::Unknown => TypeDefinition::Unknown,
            _ => {
                core::todo!(
                    "DIFTypeDefinition::to_type_definition for this variant is not implemented yet"
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DIFType {
    pub name: Option<String>,
    pub mutability: Option<ReferenceMutability>,
    pub type_definition: DIFTypeDefinition,
}
impl DIFConvertible for DIFType {}

/// DIFType serializes as normal struct - for Reference type_definition without name or mutability, the pointer
/// address is directly serialized as string
/// (same for deserialization)
impl Serialize for DIFType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.name.is_none()
            && self.mutability.is_none()
            && let DIFTypeDefinition::Reference(_) = self.type_definition
        {
            return self.type_definition.serialize(serializer);
        }

        let field_count = 1
            + if self.name.is_some() { 1 } else { 0 }
            + if self.mutability.is_some() { 1 } else { 0 };
        let mut state = serializer.serialize_struct("DIFType", field_count)?;
        if let Some(name) = &self.name {
            state.serialize_field("name", name)?;
        }
        if let Some(mutability) = &self.mutability {
            state.serialize_field("mut", mutability)?;
        }
        state.serialize_field("def", &self.type_definition)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for DIFType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        struct DIFTypeVisitor;

        impl<'de> Visitor<'de> for DIFTypeVisitor {
            type Value = DIFType;

            fn expecting(
                &self,
                formatter: &mut core::fmt::Formatter,
            ) -> core::fmt::Result {
                formatter.write_str("struct DIFType")
            }

            fn visit_str<E>(self, v: &str) -> Result<DIFType, E>
            where
                E: de::Error,
            {
                let type_def =
                    DIFTypeDefinition::deserialize(v.into_deserializer())?;
                Ok(DIFType {
                    name: None,
                    mutability: None,
                    type_definition: type_def,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<DIFType, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name: Option<String> = None;
                let mut mutability: Option<ReferenceMutability> = None;
                let mut type_definition: Option<DIFTypeDefinition> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => name = Some(map.next_value()?),
                        "mut" => mutability = Some(map.next_value()?),
                        "def" => type_definition = Some(map.next_value()?),
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["name", "mut", "def"],
                            ));
                        }
                    }
                }

                let type_definition = type_definition
                    .ok_or_else(|| de::Error::missing_field("def"))?;

                Ok(DIFType {
                    name,
                    mutability,
                    type_definition,
                })
            }
        }

        deserializer.deserialize_any(DIFTypeVisitor)
    }
}

impl DIFType {
    pub(crate) fn from_type(ty: &Type, memory: &RefCell<Memory>) -> Self {
        DIFType {
            name: None,
            mutability: ty.reference_mutability.clone(),
            type_definition: DIFTypeDefinition::from_type_definition(
                &ty.type_definition,
                memory,
            ),
        }
    }

    pub(crate) fn from_type_definition(
        type_def: &TypeDefinition,
        memory: &RefCell<Memory>,
    ) -> Self {
        DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::from_type_definition(
                type_def, memory,
            ),
        }
    }

    pub(crate) fn to_type(&self, memory: &RefCell<Memory>) -> Type {
        Type {
            reference_mutability: self.mutability.clone(),
            type_definition: self.to_type_definition(memory),
            base_type: None,
        }
    }

    pub(crate) fn to_type_definition(
        &self,
        memory: &RefCell<Memory>,
    ) -> TypeDefinition {
        DIFTypeDefinition::to_type_definition(&self.type_definition, memory)
    }
}

impl From<DIFTypeRepresentation> for DIFType {
    fn from(value: DIFTypeRepresentation) -> Self {
        DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition { value, ty: None },
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dif_type_serialization() {
        let dif_type = DIFType {
            name: Some("ExampleType".to_string()),
            mutability: Some(ReferenceMutability::Mutable),
            type_definition: DIFTypeDefinition::Unit,
        };
        let serialized = dif_type.as_json();
        println!("Serialized DIFType: {}", serialized);
        let deserialized = DIFType::from_json(&serialized);
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn object() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value: DIFTypeRepresentation::Object(vec![
                        (
                            "field1".to_string(),
                            DIFType::from(DIFTypeRepresentation::Null),
                        ),
                        (
                            "field2".to_string(),
                            DIFType::from(DIFTypeRepresentation::Number(42.0)),
                        ),
                    ]),
                    ty: None,
                },
            )),
        };
        let serialized = dif_type.as_json();
        let deserialized: DIFType = DIFType::from_json(&serialized);
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
                            )),
                            DIFType::from(DIFTypeRepresentation::Number(42.0)),
                        ),
                        (
                            DIFType::from(DIFTypeRepresentation::Number(1.0)),
                            DIFType::from(DIFTypeRepresentation::Number(3.5)),
                        ),
                    ]),
                    ty: None,
                },
            )),
        };
        let serialized = dif_type.as_json();
        let deserialized: DIFType = DIFType::from_json(&serialized);
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
                        DIFType::from(DIFTypeRepresentation::Number(1.0)),
                        DIFType::from(DIFTypeRepresentation::Number(2.0)),
                        DIFType::from(DIFTypeRepresentation::Number(3.0)),
                    ]),
                    ty: None,
                },
            )),
        };
        let serialized = dif_type.as_json();
        println!("Serialized DIFType: {}", serialized);
        let deserialized: DIFType = DIFType::from_json(&serialized);
        println!("Deserialized DIFType: {:#?}", deserialized);
        assert_eq!(dif_type, deserialized);
    }
}
