use std::fmt::Display;
use std::hash::Hasher;

use serde::{Deserialize, Serialize};

use crate::values::core_value_trait::CoreValueTrait;
use crate::values::core_values::r#type::descriptor::TypeDescriptor;
use crate::values::core_values::r#type::path::TypePath;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::ValueContainer;

#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub struct Type {
    pub name: TypePath,
    pub descriptor: TypeDescriptor,
    pub base_type: Option<TypePath>,
}

impl Type {
    pub fn new(name: impl Into<TypePath>, descriptor: TypeDescriptor) -> Self {
        Type {
            name: name.into(),
            descriptor,
            base_type: None,
        }
    }
    pub fn new_with_base(
        name: impl Into<TypePath>,
        descriptor: TypeDescriptor,
        base_type: impl Into<TypePath>,
    ) -> Self {
        Type {
            name: name.into(),
            descriptor,
            base_type: Some(base_type.into()),
        }
    }

    /// Checks if the current type is a subtype of another type.
    pub fn is_subtype_of(&self, other: &Self) -> bool {
        other.name.is_parent_of(&self.name)
    }

    /// Checks if the current type is a parent type of another type.
    pub fn is_parent_type_of(&self, other: &Self) -> bool {
        self.name.is_parent_of(&other.name)
    }

    pub fn matches(&self, value: &ValueContainer) -> bool {
        todo!("Implement type matching logic for Type::matches");
    }
}

impl CoreValueTrait for Type {}

impl StructuralEq for Type {
    fn structural_eq(&self, other: &Self) -> bool {
        self.name == other.name && self.descriptor == other.descriptor
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
