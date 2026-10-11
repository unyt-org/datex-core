//! This module contains the implementation of the [TypeDefinition] enum, which represents a underlying type definition in the DATEX type system.
//! A [TypeDefinition] can hold e.g. a [LiteralTypeDefinition], or a [CollectionTypeDefinition], or a [SharedContainerContainingType] wand impl types.
//! The [TypeDefinition] is used as the underlying structure for type definitions in the type space and is wrapped by [TypeDefinitionWithMetadata] which holds additional metadata for type checking and inference.

use strum::AsRefStr;
pub mod binrw;
use crate::{
    libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId},
    prelude::*,
    shared_values::PointerAddress,
    types::{
        literal_type_definition::LiteralTypeDefinition,
        shared_container_containing_type::SharedContainerContainingType,
        r#type::Type,
        type_definition::{
            callable::CallableTypeDefinition,
            collection::CollectionTypeDefinition, impl_type::ImplMarkers,
            intersection::IntersectionTypeDefinition, list::ListTypeDefinition,
            map::MapTypeDefinition, range::RangeTypeDefinition,
            tagged_type::TaggedTypeDefinition, union::UnionTypeDefinition,
        },
        type_definition_with_metadata::{
            TypeDefinitionWithMetadata, TypeMetadata,
        },
    },
    values::core_values::native::DatexNative,
};
use core::{fmt::Display, hash::Hash, ops::Deref, prelude::rust_2024::*};

pub mod callable;
pub mod collection;
pub mod impl_type;
pub mod list;
pub mod map;
pub mod range;
pub mod tagged_type;
pub mod type_match;

/// Base enum for a type definition
/// This is normally the base for types at runtime, in contrast to [Type], which is the base for types
/// at compile time.
#[derive(Debug, Clone, PartialEq, Eq, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum TypeDefinition {
    /// e.g. 1, "example"
    Literal(LiteralTypeDefinition),

    List(ListTypeDefinition), // e.g. [&mut integer, text, boolean]

    Map(MapTypeDefinition),

    Range(RangeTypeDefinition),

    // TODO #371: Rename to generic?
    /// e.g. [integer], [integer; 5], Map<string, integer>
    Collection(CollectionTypeDefinition),

    /// type A = {b: B} // $A
    /// type B = {a: $A}
    Shared(SharedContainerContainingType), // integer

    /// needed for nested types with multiple reference layers (e.g. 'mut 'mut shared X or (integer | null) | null)
    Box(Box<Type>),

    /// a callable type definition (signature)
    Callable(CallableTypeDefinition),

    /// innerType + Marker1 + Marker2
    /// A special type that behaves like `innerType` but is marked with additional
    /// pointer addresses that represent meta information about the type.
    /// The type is treated as equivalent to `innerType` for most operations,
    /// but the impl markers can be used to enforce additional constraints during
    /// type checking or runtime behavior.
    ImplMarkers(ImplMarkers),

    /// NOTE: all the types below can never exist as actual types of a runtime value - they are only
    /// relevant for type space definitions and type checking.

    /// A & B & C
    Intersection(IntersectionTypeDefinition),

    /// A | B | C
    Union(UnionTypeDefinition),

    /// #Tagged or #Tagged {...}
    /// #Tagged(null) is equivalent to #Tagged
    TaggedType(TaggedTypeDefinition),

    // core types ("nominal")
    CoreType(CoreLibTypeId), // -> $123
}

impl TypeDefinition {
    /// Returns true if the type definition is a structural type (e.g. a collection, literal, or shared type).
    pub fn is_structural(&self) -> bool {
        !matches!(
            self,
            TypeDefinition::CoreType(_)
                | TypeDefinition::Box(Type::Entity(_))
                | TypeDefinition::ImplMarkers(_)
        )
    }

    pub fn is_tagged(&self) -> bool {
        matches!(self, TypeDefinition::TaggedType(_))
    }
    pub fn try_unbox(&self) -> Option<&Type> {
        match self {
            TypeDefinition::Box(boxed) => Some(boxed),
            _ => None,
        }
    }
}

impl Hash for TypeDefinition {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypeDefinition::Collection(value) => {
                value.hash(state);
            }
            TypeDefinition::Literal(value) => {
                value.hash(state);
            }
            TypeDefinition::Map(map) => {
                for (key, value) in map.iter() {
                    key.hash(state);
                    value.hash(state);
                }
            }
            TypeDefinition::List(list) => {
                for element in list.iter() {
                    element.hash(state);
                }
            }
            TypeDefinition::Range(ty) => {
                ty.hash(state);
            }
            TypeDefinition::Shared(reference) => {
                reference.hash(state);
            }

            TypeDefinition::Union(types) => {
                for ty in types.iter() {
                    ty.hash(state);
                }
            }
            TypeDefinition::Intersection(types) => {
                for ty in types.iter() {
                    ty.hash(state);
                }
            }
            TypeDefinition::Callable(callable) => {
                callable.kind.hash(state);
                for (name, ty) in callable.parameters.iter() {
                    name.hash(state);
                    ty.hash(state);
                }
                callable.rest_parameter.hash(state);
                callable.return_type.hash(state);
                callable.yeet_type.hash(state);
            }
            TypeDefinition::ImplMarkers(definition) => {
                definition.hash(state);
            }
            TypeDefinition::Box(ty) => {
                ty.hash(state);
            }
            TypeDefinition::CoreType(core) => {
                core.hash(state);
            }
            TypeDefinition::TaggedType(tagged_type) => {
                tagged_type.hash(state);
            }
        }
    }
}

impl Display for TypeDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TypeDefinition::Collection(value) => {
                write!(f, "{}", value)
            }
            TypeDefinition::Map(entries) => {
                let entries_str: Vec<String> = entries
                    .0
                    .iter()
                    .map(|(key, value)| format!("{}: {}", key, value))
                    .collect();
                write!(f, "{{{}}}", entries_str.join(", "))
            }
            TypeDefinition::List(elements) => {
                let elements_str: Vec<String> =
                    elements.iter().map(|e| e.to_string()).collect();
                write!(f, "[{}]", elements_str.join(", "))
            }
            TypeDefinition::Range(range) => {
                write!(f, "{}", range)
            }

            TypeDefinition::Literal(value) => {
                write!(f, "{}", value)
            }
            TypeDefinition::Shared(reference) => {
                write!(f, "{}", reference.deref())
            }
            TypeDefinition::ImplMarkers(definition) => {
                write!(f, "{}", definition)?;
                Ok(())
            }

            TypeDefinition::Union(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", types_str.join(" | "))
            }
            TypeDefinition::Intersection(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", types_str.join(" & "))
            }
            TypeDefinition::Callable(callable) => {
                let mut params_code: Vec<String> = callable
                    .parameters
                    .iter()
                    .map(|(param_name, param_type)| match param_name {
                        Some(name) => format!("{}: {}", name, param_type),
                        None => format!("{}", param_type),
                    })
                    .collect();
                // handle rest parameter
                if let Some((param_name, param_type)) = &callable.rest_parameter
                {
                    params_code.push(match param_name {
                        Some(name) => format!("...{}: {}", name, param_type),
                        None => format!("...{}", param_type),
                    });
                }

                let return_type_code = match &callable.return_type {
                    Some(return_type) => format!(" -> {}", return_type),
                    None => " -> ()".to_string(),
                };

                let yeet_type_code = match &callable.yeet_type {
                    Some(yeet_type) => format!(" yeets {}", yeet_type),
                    None => "".to_string(),
                };

                write!(
                    f,
                    "{} ({}){}{}",
                    callable.kind,
                    params_code.join(", "),
                    return_type_code,
                    yeet_type_code
                )
            }
            TypeDefinition::Box(ty) => {
                write!(f, "{}", ty)
            }
            TypeDefinition::CoreType(core) => {
                write!(f, "{}", core)
            }
            TypeDefinition::TaggedType(tagged_type) => {
                write!(f, "{}", tagged_type)
            }
        }
    }
}

pub mod equality;
pub mod intersection;
mod serde_dif;
#[cfg(feature = "ast")]
mod to_type_expression_data;
pub mod union;

impl TypeDefinition {
    pub const UNIT: TypeDefinition =
        TypeDefinition::CoreType(CoreLibTypeId::Base(CoreLibBaseTypeId::Unit));

    pub const NULL: TypeDefinition =
        TypeDefinition::CoreType(CoreLibTypeId::Base(CoreLibBaseTypeId::Null));

    /// Calls the provided callback with a reference to the recursively collapsed inner [TypeDefinition] value
    pub fn with_collapsed<R>(&self, f: impl FnOnce(&TypeDefinition) -> R) -> R {
        match self {
            TypeDefinition::Shared(reference) =>
            // collapse shared container to inner Type
            {
                reference.with_collapsed_type_value(|ty| {
                    // collapse Type definition to inner type definition
                    ty.with_collapsed_type_definition(f)
                })
            }
            _ => f(self),
        }
    }

    /// Creates a new core type definition.
    pub fn core(id: impl Into<CoreLibTypeId>) -> TypeDefinition {
        TypeDefinition::CoreType(id.into())
    }

    /// Creates a new literal type.
    pub fn literal(literal_type: impl Into<LiteralTypeDefinition>) -> Self {
        TypeDefinition::Literal(literal_type.into())
    }

    /// Creates a new list type.
    pub fn list(element_types: Vec<Type>) -> Self {
        TypeDefinition::List(ListTypeDefinition(element_types))
    }

    /// Creates a new union type.
    pub fn union<T>(types: Vec<T>) -> Self
    where
        T: Into<Type>,
    {
        let types = types.into_iter().map(|t| t.into()).collect();
        TypeDefinition::Union(types)
    }

    /// Creates a new intersection type.
    pub fn intersection<T>(types: Vec<T>) -> Self
    where
        T: Into<Type>,
    {
        let types = types.into_iter().map(|t| t.into()).collect();
        TypeDefinition::Intersection(types)
    }

    /// Creates a new shared type.
    pub fn shared(ty: SharedContainerContainingType) -> Self {
        TypeDefinition::Shared(ty)
    }

    /// Creates a new callable type.
    pub fn callable(definition: CallableTypeDefinition) -> Self {
        TypeDefinition::Callable(definition)
    }

    /// Creates a new type with impls.
    pub fn impl_markers(impls: Vec<PointerAddress>) -> Self {
        TypeDefinition::ImplMarkers(ImplMarkers::new(impls))
    }

    /// Get the core lib type pointer id for this structural type definition
    pub fn core_lib_type_id(&self) -> Option<CoreLibTypeId> {
        match self {
            TypeDefinition::Literal(literal_definition) => {
                Some(literal_definition.core_lib_type_id())
            }
            TypeDefinition::List(_) => {
                Some(CoreLibTypeId::Base(CoreLibBaseTypeId::List))
            }
            TypeDefinition::Map(_) => {
                Some(CoreLibTypeId::Base(CoreLibBaseTypeId::Map))
            }
            TypeDefinition::Range(_) => {
                Some(CoreLibTypeId::Base(CoreLibBaseTypeId::Range))
            }
            TypeDefinition::Callable(_) => {
                Some(CoreLibTypeId::Base(CoreLibBaseTypeId::Callable))
            }
            TypeDefinition::CoreType(id) => Some(*id),
            _ => None,
        }
    }

    /// Convert this type definition into a [Type] by wrapping it in a [Type::Definition] variant.
    /// If the type definition is a [TypeDefinition::Container] variant, it will be unwrapped and returned as the inner [Type].
    pub fn convert_to_type(self) -> Type {
        match self {
            TypeDefinition::Box(ty) => *ty,
            _ => Type::Definition(self.into()),
        }
    }
}

impl TypeDefinition {
    /// Map a type definition (e.g. 42u8) to it's upper level base type (e.g. integer)
    /// integer/u8 -> integer
    /// integer -> integer
    /// 42u8 -> integer
    /// 42 -> integer
    /// User/variant -> User
    pub fn base_core_lib_type(&self) -> CoreLibTypeId {
        match &self {
            TypeDefinition::Literal(value) => value.core_lib_type_id(),
            TypeDefinition::Union(_) => {
                core::todo!("#322 handle union base type"); // generic type base type / type
            }
            TypeDefinition::Shared(reference) => reference
                .with_collapsed_type_value(|ty| ty.base_core_lib_type()),
            _ => core::panic!("Unhandled type definition for base type"),
        }
    }
}

impl From<TypeDefinition> for TypeDefinitionWithMetadata {
    fn from(structural_definition: TypeDefinition) -> Self {
        TypeDefinitionWithMetadata::new(
            structural_definition,
            TypeMetadata::default(),
        )
    }
}

impl From<Type> for TypeDefinitionWithMetadata {
    fn from(ty: Type) -> Self {
        match ty {
            Type::Definition(definition) => definition,
            _ => ty.convert_to_definition().into(),
        }
    }
}
impl From<Type> for TypeDefinition {
    fn from(ty: Type) -> Self {
        match ty {
            Type::Definition(definition)
                if definition.has_default_metadata() =>
            {
                definition.definition
            }
            _ => ty.convert_to_definition(),
        }
    }
}

impl From<LiteralTypeDefinition> for TypeDefinitionWithMetadata {
    fn from(literal_definition: LiteralTypeDefinition) -> Self {
        TypeDefinitionWithMetadata::new(
            literal_definition.into(),
            TypeMetadata::default(),
        )
    }
}
