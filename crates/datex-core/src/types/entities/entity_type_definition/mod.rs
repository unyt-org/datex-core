#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod value_access;

use super::entity_impls::{EntityImpl, EntityImplMethod};
use crate::{
    prelude::*, types::type_definition::TypeDefinition,
    values::core_values::callable::Callable,
};
mod to_instructions;
pub mod dyn_deserialize;

use core::fmt::Display;
use crate::types::entities::entity_type_definition::dyn_deserialize::DeFn;

/// Represents a definition of an "entity" type,
/// which describes a nominal type identified by a unique pointer id.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct EntityTypeDefinition {
    pub(crate) definition: TypeDefinition,
    pub(crate) name: String,
    pub(crate) allowed_variants: Vec<String>,
    pub(crate) impls: Vec<EntityImpl>,
    // TODO: dif feature flag?
    pub(crate) deserialize_from_dif: Option<DeFn>,
}

impl Display for EntityTypeDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl EntityTypeDefinition {
    pub fn new(
        definition: TypeDefinition,
        name: String,
    ) -> EntityTypeDefinition {
        EntityTypeDefinition {
            definition,
            name,
            allowed_variants: Vec::new(),
            impls: Vec::new(),
            deserialize_from_dif: None,
        }
    }

    pub fn new_with_impls(
        definition: TypeDefinition,
        name: String,
        impls: Vec<EntityImpl>,
    ) -> EntityTypeDefinition {
        EntityTypeDefinition {
            definition,
            name,
            allowed_variants: Vec::new(),
            impls,
            deserialize_from_dif: None,
        }
    }
    
    pub fn new_with_impls_and_deserialize(
        definition: TypeDefinition,
        name: String,
        impls: Vec<EntityImpl>,
        deserialize_from_dif: Option<DeFn>,
    ) -> EntityTypeDefinition {
        EntityTypeDefinition {
            definition,
            name,
            allowed_variants: Vec::new(),
            impls,
            deserialize_from_dif,
        }
    }

    pub fn impls(&self) -> &[EntityImpl] {
        &self.impls
    }
    
    pub fn deserialize_from_dif(&self) -> Option<&DeFn> {
        self.deserialize_from_dif.as_ref()
    }

    /// Returns a reference to the method for the given method name, if it exists in this implementation.
    pub fn try_get_method(
        &self,
        method_name: &str,
    ) -> Option<&EntityImplMethod> {
        self.impls
            .iter()
            .filter_map(|impl_ty| impl_ty.try_get_method(method_name))
            .next()
    }

    /// Returns a reference to the (static) method for the given property name, if it exists in this implementation.
    pub fn try_get_property(&self, property_name: &str) -> Option<&Callable> {
        self.impls
            .iter()
            .filter_map(|impl_ty| impl_ty.try_get_property(property_name))
            .next()
    }
}

impl EntityTypeDefinition {
    /// Get the inner [TypeDefinition]
    pub fn definition(&self) -> &TypeDefinition {
        &self.definition
    }

    /// Replace the inner [TypeDefinition] with a new one and return the old one
    pub fn replace_definition(&mut self, new_definition: TypeDefinition) {
        self.definition = new_definition;
    }
}
