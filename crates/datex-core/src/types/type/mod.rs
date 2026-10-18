//! This module contains the implementation of the [Type] enum, which represents a type in the DATEX type system.
//! A [Type] can either be an alias to a [TypeDefinitionWithMetadata] or a nominal type represented by a [EntityType].

#[cfg(feature = "compiler")]
use crate::ast::expressions::DatexExpressionData;
use crate::{
    libs::core::type_id::CoreLibTypeId,
    prelude::*,
    runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
    shared_values::{
        ReferenceMutability, SharedContainerMutability,
        SharedContainerOwnership,
    },
    types::{
        entities::entity_type_definition::EntityTypeDefinition,
        entity_type::EntityType,
        literal_type_definition::LiteralTypeDefinition,
        type_definition::TypeDefinition,
        type_definition_with_metadata::{
            LocalMutability, LocalOwnership, TypeDefinitionWithMetadata,
            TypeMetadata,
        },
    },
    values::{core_value::CoreValue, value_container::ValueContainer},
};
use core::{fmt::Display, hash::Hash, ops::Deref};

pub mod type_match;

/// Base enum for a type
/// This is normally the base for types at compile time, in contrast to [TypeDefinition], which is the base for types
/// at runtime.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Type {
    /// A type definition with metadata, corresponding to `type x = X`. Treated as a structural type
    Definition(TypeDefinitionWithMetadata),
    /// A nominal type, corresponding to `entity x = X`.
    Entity(EntityType),
}

impl Type {
    pub const UNIT: Type = Type::Definition(TypeDefinitionWithMetadata::unit());
    pub const NULL: Type = Type::Definition(TypeDefinitionWithMetadata::null());

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        match &mut self {
            Type::Definition(alias) => {
                alias.set_reference_name(name.into());
            }
            _ => unimplemented!(
                "Naming is only supported for alias types for now"
            ),
        }
        self
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Type::Definition(alias) => alias.reference_name(),
            Type::Entity(_) => None,
        }
    }

    pub fn entity(
        definition: EntityTypeDefinition,
        address_provider: &mut SelfOwnedPointerAddressProvider,
    ) -> Type {
        Type::Entity(EntityType::new_from_definition(
            definition,
            address_provider,
        ))
    }

    /// Creates a new core type
    pub fn core(id: impl Into<CoreLibTypeId>) -> Type {
        Type::Definition(TypeDefinition::core(id).into())
    }

    /// Checks if the type is a simple alias to a core library type with default local metadata
    pub fn is_core_lib_type(&self) -> bool {
        self.try_as_core_lib_type().is_some()
    }

    /// Collapses nominal type definitions to their underlying type definitions with metadata
    pub fn as_definition_with_metadata<R>(
        &self,
        f: impl FnOnce(&TypeDefinitionWithMetadata) -> R,
    ) -> R {
        match self {
            Type::Definition(type_def) => f(type_def),
            Type::Entity(nominal_def) => {
                f(&(nominal_def.entity_definition().definition.clone().into()))
            }
        }
    }

    /// Collapses nominal type definitions to their underlying structural type definitions
    pub fn with_collapsed_type_definition<R>(
        &self,
        f: impl FnOnce(&TypeDefinition) -> R,
    ) -> R {
        self.as_definition_with_metadata(|def| f(&def.definition))
    }

    pub fn base_core_lib_type(&self) -> CoreLibTypeId {
        match self {
            Type::Definition(type_def) => {
                type_def.definition.base_core_lib_type()
            }
            Type::Entity(_nominal_def) => {
                todo!()
            }
        }
    }

    /// Boxes the type in a new [TypeDefinition::Container] with the provided metadata.
    /// If the type is already a transparent wrapper (alias) with local metadata, it just updates the metadata without adding another nesting layer.
    pub fn box_with_metadata(self, metadata: TypeMetadata) -> Type {
        match self {
            // if simple transparent with default local metadata, just update metadata without adding another nesting layer
            Type::Definition(TypeDefinitionWithMetadata {
                metadata:
                    TypeMetadata::Local {
                        ownership: LocalOwnership::Owned,
                        mutability: LocalMutability::Immutable,
                    },
                definition,
                ..
            }) => Type::Definition(TypeDefinitionWithMetadata::new(
                definition, metadata,
            )),
            // box otherwise
            _ => Type::Definition(TypeDefinitionWithMetadata::new(
                TypeDefinition::Box(Box::new(self)),
                metadata,
            )),
        }
    }

    /// Tries to convert the type into a shared reference type with the provided reference mutability.
    ///
    /// This only succeeds if the type is a already a type with [TypeMetadata::Shared] and the provided reference mutability is compatible with the ownership and mutability of the shared container.
    pub fn try_convert_to_shared_ref(
        self,
        reference_mutability: ReferenceMutability,
    ) -> Result<Type, ()> {
        match self {
            // if simple transparent with default local metadata, just update metadata without adding another nesting layer
            Type::Definition(TypeDefinitionWithMetadata {
                metadata:
                    TypeMetadata::Shared {
                        ownership,
                        mutability,
                    },
                definition,
                ..
            }) => {
                // max mutability that is allowed for the reference is determined by ownership and mutability of the shared container
                let max_mutability = match &ownership {
                    SharedContainerOwnership::Owned => match mutability {
                        SharedContainerMutability::Immutable => {
                            ReferenceMutability::Immutable
                        }
                        SharedContainerMutability::Mutable => {
                            ReferenceMutability::Mutable
                        }
                    },
                    SharedContainerOwnership::Referenced(
                        reference_mutability,
                    ) => *reference_mutability,
                };

                if reference_mutability <= max_mutability {
                    Ok(Type::Definition(TypeDefinitionWithMetadata::new(
                        definition,
                        TypeMetadata::Shared {
                            ownership: SharedContainerOwnership::Referenced(
                                reference_mutability,
                            ),
                            mutability,
                        },
                    )))
                } else {
                    Err(())
                }
            }
            // box otherwise
            _ => Err(()),
        }
    }

    /// Converts the given [Type] to an equivalent [TypeDefinition]
    pub fn convert_to_definition(self) -> TypeDefinition {
        // just collapse to definition
        if let Type::Definition(TypeDefinitionWithMetadata { metadata, .. }) =
            &self
            && metadata == &TypeMetadata::default()
        {
            match self {
                Type::Definition(TypeDefinitionWithMetadata {
                    metadata: _,
                    definition,
                    ..
                }) => definition,
                _ => unreachable!(),
            }
        }
        // nest type
        else {
            TypeDefinition::Box(Box::new(self))
        }
    }

    /// Tries to extract the core library type id if the type is a simple alias to a core library type with default local metadata.
    pub fn try_as_core_lib_type(&self) -> Option<CoreLibTypeId> {
        match self {
            Type::Definition(TypeDefinitionWithMetadata {
                definition: TypeDefinition::CoreType(core_lib_type_id),
                metadata,
                ..
            }) if metadata == &TypeMetadata::default() => {
                Some(*core_lib_type_id)
            }
            _ => None,
        }
    }
}

impl From<TypeDefinitionWithMetadata> for Type {
    fn from(definition_with_metadata: TypeDefinitionWithMetadata) -> Self {
        if definition_with_metadata.has_default_metadata() {
            definition_with_metadata.definition.convert_to_type()
        } else {
            Type::Definition(definition_with_metadata)
        }
    }
}
impl From<TypeDefinition> for Type {
    fn from(definition: TypeDefinition) -> Self {
        definition.convert_to_type()
    }
}

impl Type {
    // / 1 matches 1 -> true
    // / 1 matches 2 -> false
    // / 1 matches 1 | 2 -> true
    // / 1 matches "x" | 2 -> false
    // / integer matches 1 | 2 -> false
    // pub fn value_matches(&self, value: &ValueContainer) -> bool {
    //     Type::value_matches_type(value, self)
    // }

    // / Checks if an atomic type matches another type
    // / An atomic type can be any type variant besides union or intersection
    // pub fn atomic_matches_type(atomic_type: &Type, other: &Type) -> bool {
    //     // FIXME #768: match rules for prefixes are more nuanced than just equality, e.g. &mut T should match &T, ...
    //     if atomic_type.metadata != other.metadata {
    //         return false;
    //     }

    //     match &other.type_definition {
    //         TypeDefinition::Shared(reference) => {
    //             // compare base type of atomic_type with the referenced type
    //             if let Some(atomic_base_type_reference) =
    //                 atomic_type.base_type_reference()
    //             {
    //                 *atomic_base_type_reference.borrow() == *reference.borrow()
    //             } else {
    //                 false
    //             }
    //         }
    //         TypeDefinition::Union(members) => {
    //             // atomic type must match at least one member of the union
    //             for member in members {
    //                 if Type::atomic_matches_type(atomic_type, member) {
    //                     return true;
    //                 }
    //             }
    //             false
    //         }
    //         TypeDefinition::Intersection(members) => {
    //             // atomic type must match all members of the intersection
    //             for member in members {
    //                 if !Type::atomic_matches_type(atomic_type, member) {
    //                     return false;
    //                 }
    //             }
    //             true
    //         }
    //         _ => {
    //             // compare type definitions directly
    //             atomic_type.type_definition == other.type_definition
    //         }
    //     }
    // }

    // /// Matches a value against a type
    // pub fn value_matches_type(
    //     value: &ValueContainer,
    //     match_type: &Type,
    // ) -> bool {
    //     // if match_type == &value.actual_type().as_type() {
    //     //     return true;
    //     // }

    //     match &match_type.type_definition {
    //         // e.g. 1 matches 1 | 2
    //         TypeDefinition::Union(types) => {
    //             // value must match at least one of the union types
    //             types.iter().any(|t| Type::value_matches_type(value, t))
    //         }
    //         TypeDefinition::Intersection(types) => {
    //             // value must match all of the intersection types
    //             types.iter().all(|t| Type::value_matches_type(value, t))
    //         }
    //         TypeDefinition::Literal(structural_type) => {
    //             structural_type.value_matches(value)
    //         }
    //         TypeDefinition::Shared(_reference) => {
    //             core::todo!("#327 handle reference type matching");
    //             //reference.value_matches(value)
    //         }
    //         TypeDefinition::Type(inner_type) => {
    //             // TODO #464: also check mutability of current type?
    //             inner_type.value_matches(value)
    //         }
    //         TypeDefinition::Callable(_signature) => {
    //             core::todo!("#328 handle function type matching");
    //         }
    //         TypeDefinition::Collection(_collection_type) => {
    //             core::todo!("#329 handle collection type matching");
    //         }
    //         TypeDefinition::Unit => false, // unit type does not match any value
    //         TypeDefinition::Never => false,
    //         TypeDefinition::Unknown => false,
    //         TypeDefinition::ImplType(ty, _) => {
    //             Type::value_matches_type(value, ty)
    //         }
    //     }
    // }
}

mod convert_parts;
mod datex_hash;
mod datex_native;
mod datex_native_structural;
pub mod equality;
mod get_core_lib_type_id;
pub mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
#[cfg(feature = "ast")]
mod to_type_expression_data;
mod value_access;
pub mod get_datex_type;

impl Display for Type {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Type::Definition(def) => write!(f, "{}", def),
            Type::Entity(nom) => write!(f, "{}", nom.deref()),
        }
    }
}

// impl From<&CoreValue> for Type {
//     fn from(value: &CoreValue) -> Self {
//         match value {
//             CoreValue::Null => Type::structural(
//                 LiteralTypeDefinition::Null,
//                 TypeMetadata::default(),
//             ),
//             CoreValue::Boolean(b) => Type::structural(
//                 LiteralTypeDefinition::Boolean(b.clone()),
//                 TypeMetadata::default(),
//             ),
//             CoreValue::Text(s) => {
//                 Type::structural(s.clone(), TypeMetadata::default())
//             }
//             CoreValue::Decimal(d) => Type::structural(
//                 LiteralTypeDefinition::Decimal(d.clone()),
//                 TypeMetadata::default(),
//             ),
//             CoreValue::TypedDecimal(td) => Type::structural(
//                 LiteralTypeDefinition::TypedDecimal(td.clone()),
//                 TypeMetadata::default(),
//             ),
//             CoreValue::Integer(i) => Type::structural(
//                 LiteralTypeDefinition::Integer(i.clone()),
//                 TypeMetadata::default(),
//             ),
//             CoreValue::TypedInteger(ti) => Type::structural(
//                 LiteralTypeDefinition::TypedInteger(ti.clone()),
//                 TypeMetadata::default(),
//             ),
//             CoreValue::Endpoint(e) => Type::structural(
//                 LiteralTypeDefinition::Endpoint(e.clone()),
//                 TypeMetadata::default(),
//             ),
//             CoreValue::List(list) => {
//                 let types = list
//                     .iter()
//                     .map(|v| {
//                         Type::from(v.to_cloned_value().borrow().inner.clone())
//                     })
//                     .collect::<Vec<_>>();
//                 Type::structural(
//                     LiteralTypeDefinition::List(types),
//                     TypeMetadata::default(),
//                 )
//             }
//             CoreValue::Map(map) => {
//                 let struct_types = map
//                     .iter()
//                     .map(|(key, value)| {
//                         (
//                             Type::from(
//                                 ValueContainer::from(key)
//                                     .to_cloned_value()
//                                     .borrow()
//                                     .inner
//                                     .clone(),
//                             ),
//                             Type::from(
//                                 value.to_cloned_value().borrow().inner.clone(),
//                             ),
//                         )
//                     })
//                     .collect::<Vec<_>>();
//                 Type::structural(
//                     LiteralTypeDefinition::Map(struct_types),
//                     TypeMetadata::default(),
//                 )
//             }
//             e => unimplemented!("Type conversion not implemented for {}", e),
//         }
//     }
// }
// impl From<CoreValue> for Type {
//     fn from(value: CoreValue) -> Self {
//         Type::from(&value)
//     }
// }

#[cfg(feature = "compiler")]
impl TryFrom<&DatexExpressionData> for LiteralTypeDefinition {
    type Error = ();

    fn try_from(expr: &DatexExpressionData) -> Result<Self, Self::Error> {
        Ok(match expr {
            DatexExpressionData::Boolean(b) => {
                LiteralTypeDefinition::Boolean(b.clone())
            }
            DatexExpressionData::Text(s) => {
                LiteralTypeDefinition::Text(s.clone())
            }
            DatexExpressionData::Decimal(d) => {
                LiteralTypeDefinition::Decimal(d.clone())
            }
            DatexExpressionData::Integer(i) => {
                LiteralTypeDefinition::Integer(i.clone())
            }
            DatexExpressionData::Endpoint(e) => {
                LiteralTypeDefinition::Endpoint(e.clone())
            }
            _ => return Err(()),
        })
    }
}

#[cfg(feature = "compiler")]
impl TryFrom<&DatexExpressionData> for Type {
    type Error = ();

    fn try_from(expr: &DatexExpressionData) -> Result<Self, Self::Error> {
        Ok(Type::from(LiteralTypeDefinition::try_from(expr)?))
    }
}

impl TryFrom<ValueContainer> for Type {
    type Error = ();

    fn try_from(value: ValueContainer) -> Result<Self, Self::Error> {
        match value {
            ValueContainer::Shared(shared) => {
                EntityType::try_from(shared).map(Type::Entity)
            }
            ValueContainer::Local(value) => match value.try_into_core_value() {
                Ok(CoreValue::Type(ty)) => Ok(ty),
                _ => Err(()),
            },
        }
    }
}

impl From<LiteralTypeDefinition> for Type {
    fn from(literal_type: LiteralTypeDefinition) -> Self {
        TypeDefinition::Literal(literal_type).into()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    use crate::{
        libs::core::type_id::CoreLibBaseTypeId,
        runtime::cache::shared_references_cache::SharedReferencesCache,
        types::{
            literal_type_definition::LiteralTypeDefinition,
            traits::type_match::{
                TypeSatisfiesValueContainer, TypeSubset, TypeSuperset,
            },
            r#type::Type,
            type_definition::{TypeDefinition, union::UnionTypeDefinition},
            type_definition_with_metadata::TypeDefinitionWithMetadata,
        },
        values::{
            core_value::CoreValue,
            core_values::{
                integer::{Integer, typed_integer::TypedInteger},
                text::Text,
            },
            value::Value,
            value_container::ValueContainer,
        },
    };

    #[test]
    fn match_equal_values() {
        // 1u8 matches 1u8
        assert!(
            Type::from(LiteralTypeDefinition::TypedInteger(1u8.into()))
                .satisfies_value_container(&TypedInteger::from(1u8).into())
        );

        // 1u16 matches 1u16
        assert!(
            Type::from(LiteralTypeDefinition::TypedInteger(1u16.into()))
                .satisfies_value_container(&TypedInteger::from(1u16).into())
        );

        // 1 matches 1
        assert!(
            Type::from(LiteralTypeDefinition::Integer(1.into()))
                .satisfies_value_container(&Integer::from(1).into())
        );

        // "test" matches "test"
        assert!(
            Type::from(LiteralTypeDefinition::Text("test".into()))
                .satisfies_value_container(&Text::from("test").into())
        );
    }

    #[test]
    fn match_union() {
        // 1 matches integer

        // 1 matches (1 | 2 | 3)
        assert!(
            Type::from(TypeDefinition::Union(UnionTypeDefinition(vec![
                LiteralTypeDefinition::Integer(Integer::from(1)).into(),
                LiteralTypeDefinition::Integer(Integer::from(2)).into(),
                LiteralTypeDefinition::Integer(Integer::from(3)).into()
            ])))
            .satisfies_value_container(&Integer::from(1).into())
        );
    }

    #[test]
    fn type_matches_union_type() {
        // 1 <= (1 | 2 | 3)
        assert!(
            Type::from(LiteralTypeDefinition::Integer(Integer::from(1)))
                .is_subset_of(&Type::from(TypeDefinition::Union(
                    UnionTypeDefinition(vec![
                        LiteralTypeDefinition::Integer(Integer::from(1)).into(),
                        LiteralTypeDefinition::Integer(Integer::from(2)).into(),
                        LiteralTypeDefinition::Integer(Integer::from(3)).into()
                    ])
                )))
        );

        // 1 <= integer | text
        assert!(
            Type::from(LiteralTypeDefinition::Integer(Integer::from(1)))
                .is_subset_of(&Type::from(TypeDefinition::Union(
                    UnionTypeDefinition(vec![
                        Type::core(CoreLibBaseTypeId::Integer),
                        Type::core(CoreLibBaseTypeId::Text),
                    ])
                )))
        );
    }

    // TODO #330
    // #[test]
    // fn match_combined_type() {
    //     // [1, 1] matches List<1>
    //     assert!(Type::value_matches_type(
    //         &ValueContainer::from(List::from(vec![1, 1])),
    //         &Type::list(Type::structural(1))
    //     ));
    //
    //     // [1, 2] matches List<(1 | 2)>
    //     assert!(Type::value_matches_type(
    //         &ValueContainer::from(List::from(vec![1, 2])),
    //         &Type::list(Type::union(vec![
    //             Type::structural(1).as_type_container(),
    //             Type::structural(2).as_type_container(),
    //         ])),
    //     ));
    //
    //     // [1, 2] does not match List<1>
    //     assert!(!Type::value_matches_type(
    //         &ValueContainer::from(List::from(vec![1, 2])),
    //         &Type::list(Type::structural(1))
    //     ));
    //
    //     // ["test", "jonas"] matches List<("jonas" | "test" | 3)>
    //     assert!(Type::value_matches_type(
    //         &ValueContainer::from(List::from(vec!["test", "jonas"])),
    //         &Type::list(Type::union(vec![
    //             Type::structural("jonas"),
    //             Type::structural("test"),
    //             Type::structural(3),
    //         ])),
    //     ));
    // }
}
