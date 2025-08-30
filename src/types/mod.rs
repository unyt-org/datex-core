
use std::fmt::Display;
use crate::libs::core::CoreLibPointerId;
use crate::runtime::memory::Memory;
use crate::values::core_value::CoreValue;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NominalTypeIdentifier {
    pub name: String,
    pub path: Option<String>,
}

// New type implementation based on ValueContainer
// The TypeNew struct is only a helper struct that is used by the runtime or compiler.
// The actual type is fully represented by the ValueContainer definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeNew {
    /// Optional identifier for the type, if this is a nominal type, not a structural type.
    pub nominal_identifier: Option<NominalTypeIdentifier>,
    /// Value container that defines the type.
    pub definition: ValueContainer,
}

#[derive(Debug)]
pub enum IllegalTypeError {
    MutableRef(String),
}

impl Display for IllegalTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IllegalTypeError::MutableRef(val) => write!(f, "Cannot use mutable reference as type: {}", val),
        }
    }
}

impl TryFrom<ValueContainer> for TypeNew {
    type Error = IllegalTypeError;

    // TODO: for now, we accept any ValueContainer as a TypeNew. This might be restricted later.
    // For example, mutable references should not be allowed as types.
    fn try_from(value: ValueContainer) -> Result<Self, Self::Error> {

        // if value is a TypeTag and value is a reference, this is a nominal type - assign nominal_identifier
        let nominal_identifier = if let ValueContainer::Reference(reference) = &value
            && let CoreValue::TypeTag(tag) = &reference.data.borrow().resolve_current_value().borrow().inner {
                Some(NominalTypeIdentifier {
                    name: tag.name.clone(),
                    path: None, // TODO: add path to TypeTag
                })
        }
        else {
            None
        };

        Ok(TypeNew {
            nominal_identifier,
            definition: value,
        })
    }
}

impl TypeNew {
    /// Converts a specific type (e.g. 42u8) to its base type (e.g. integer)
    pub fn get_base_type(&self, memory: &Memory) -> TypeNew {
        // check if already a base type, return self
        if self.nominal_identifier.is_some() {
            return self.clone();
        }

        // convert more specific value to its base type
        match &self.definition.to_value().borrow().inner {
            CoreValue::Integer(_) => {
                memory.get_core_type_unchecked(CoreLibPointerId::Integer)
            }
            CoreValue::Decimal(_) => {
                memory.get_core_type_unchecked(CoreLibPointerId::Decimal)
            }
            CoreValue::Boolean(_) => {
                memory.get_core_type_unchecked(CoreLibPointerId::Boolean)
            }
            _ => todo!(),
        }
    }

    /// Converts a specific  type (e.g. 42u8) to its base variant type (e.g. integer/u8)
    pub fn get_base_variant_type(&self, memory: &Memory) -> TypeNew {
        match &self.definition {
            _ => todo!(),
        }
    }

    pub fn value_matches(&self, value: &ValueContainer) -> bool {
        // TODO: implement matching logic here
        false
    }
}