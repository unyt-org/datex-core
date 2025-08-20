use std::fmt::Display;
use std::hash::Hasher;

use serde::{Deserialize, Serialize};

use crate::values::core_value::CoreValue;
use crate::values::core_value_trait::CoreValueTrait;
use crate::values::core_values::r#type::descriptor::TypeDescriptor;
use crate::values::core_values::r#type::path::TypePath;
use crate::values::datex_type::CoreValueType;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::{ValueContainer, ValueError};

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

    pub fn is_typeof(&self, other: &Self) -> bool {
        self == other || self.is_subtype_of(other)
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
        write!(f, "{}", self.name.to_clean_string())
    }
}

impl<T: Into<ValueContainer>> TryFrom<Option<T>> for Type {
    type Error = ValueError;
    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        match value {
            Some(v) => {
                let boolean: ValueContainer = v.into();
                boolean
                    .to_value()
                    .borrow()
                    .cast_to_type()
                    .ok_or(ValueError::TypeConversionError)
            }
            None => Err(ValueError::IsVoid),
        }
    }
}

impl From<Type> for CoreValue {
    fn from(value: Type) -> Self {
        CoreValue::Type(Box::new(value))
    }
}
