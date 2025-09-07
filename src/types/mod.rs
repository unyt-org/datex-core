// deprecated - use Type in types instead
use crate::libs::core::CoreLibPointerId;
use crate::runtime::memory::Memory;
use crate::values::core_value::CoreValue;
use crate::values::value_container::ValueContainer;
use std::fmt::Display;

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
            IllegalTypeError::MutableRef(val) => {
                write!(f, "Cannot use mutable reference as type: {}", val)
            }
        }
    }
}

impl TryFrom<ValueContainer> for TypeNew {
    type Error = IllegalTypeError;

    // TODO: for now, we accept any ValueContainer as a TypeNew. This might be restricted later.
    // For example, mutable references should not be allowed as types.
    fn try_from(value: ValueContainer) -> Result<Self, Self::Error> {
        // if value is a TypeTag and value is a reference, this is a nominal type - assign nominal_identifier
        let nominal_identifier = if let ValueContainer::Reference(reference) =
            &value
            && let CoreValue::TypeTag(tag) = &reference
                .data
                .borrow()
                .resolve_current_value()
                .borrow()
                .inner
        {
            Some(NominalTypeIdentifier {
                name: tag.name.clone(),
                path: None, // TODO: add path to TypeTag
            })
        } else {
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
        todo!()
    }

    // NOTE: this function currently operates in type space (type matches type, not value matches type)
    // cannot be directly used for x matches y checks in runtime, but is currently used there nevertheless
    /// Matches a value against self
    /// Returns true if all possible realizations of the value match the type
    /// Examples:
    /// 1 matches 1 -> true
    /// 1 matches 2 -> false
    /// 1 matches 1 | 2 -> true
    /// 1 matches "x" | 2 -> false
    /// 1 | 2 matches integer -> true
    /// integer matches 1 | 2 -> false
    pub fn value_matches(&self, value: &ValueContainer) -> bool {
        TypeNew::value_matches_type(value, &self.definition)
    }

    /// Matches a value against a v_type ValueContainer, which must be guaranteed to by a valid type
    fn value_matches_type(
        value: &ValueContainer,
        v_type: &ValueContainer,
    ) -> bool {
        // TODO: handle value types here
        match &value.to_value().borrow().inner {
            // each possible value of a union type must match the type
            CoreValue::Union(union) => union
                .options
                .iter()
                .all(|option| TypeNew::value_matches_type(option, v_type)),
            _ => {
                match &v_type.to_value().borrow().inner {
                    // union type matches if any of its options match
                    CoreValue::Union(union) => union
                        .options
                        .iter()
                        .any(|option| Self::value_matches_type(value, option)),
                    _ => {
                        // atomic types match if their ValueContainer types are the same
                        TypeNew::value_matches_atomic_type(value, v_type)
                    }
                }
            }
        }
    }

    /// Matches a value against an atomic type (no intersection or union type)
    pub fn value_matches_atomic_type(
        value: &ValueContainer,
        atomic_type: &ValueContainer,
    ) -> bool {
        if value == atomic_type {
            true
        }
        // check if value matches type base (e.g. 1 matches integer)
        else if let ValueContainer::Reference(reference) = atomic_type
            && let Some(pointer_id) = reference.pointer_id()
        {
            match CoreLibPointerId::from(&pointer_id) {
                CoreLibPointerId::Integer => matches!(
                    value.to_value().borrow().inner,
                    CoreValue::Integer(_)
                ),
                CoreLibPointerId::Decimal => matches!(
                    value.to_value().borrow().inner,
                    CoreValue::Decimal(_)
                ),
                CoreLibPointerId::Boolean => matches!(
                    value.to_value().borrow().inner,
                    CoreValue::Boolean(_)
                ),
                CoreLibPointerId::Text => matches!(
                    value.to_value().borrow().inner,
                    CoreValue::Text(_)
                ),
                CoreLibPointerId::Null => {
                    matches!(value.to_value().borrow().inner, CoreValue::Null)
                }
                CoreLibPointerId::Endpoint => matches!(
                    value.to_value().borrow().inner,
                    CoreValue::Endpoint(_)
                ),
                _ => false,
            }
        } else {
            false
        }
    }
}
