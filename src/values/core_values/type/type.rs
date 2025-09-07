use crate::values::core_value::CoreValue;
use crate::values::core_value_trait::CoreValueTrait;
use crate::values::reference::{Reference, ReferenceMutability};
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::ValueContainer;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NominalTypeDeclaration {
    pub name: String,
    pub variant: Option<String>,
    pub definition: Box<Reference>,
}

impl Display for NominalTypeDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(variant) = &self.variant {
            write!(f, "{}/{}", self.name, variant)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl StructuralEq for NominalTypeDeclaration {
    fn structural_eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.variant == other.variant
            && self.definition.structural_eq(&other.definition)
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Type {
    pub type_definition: TypeDefinition,
    pub reference_mutability: Option<ReferenceMutability>,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TypeDefinition {
    // {x: integer, y: text}
    Structural(Box<ValueContainer>),
    // integer or integer/u8
    Nominal(NominalTypeDeclaration),
    // e.g. A | B | C
    Union(Vec<Type>),
}

impl Display for TypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeDefinition::Structural(value) => write!(f, "{}", value),
            TypeDefinition::Nominal(nominal) => write!(f, "{}", nominal),
            TypeDefinition::Union(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", types_str.join(" | "))
            }
        }
    }
}

impl StructuralEq for TypeDefinition {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeDefinition::Structural(a), TypeDefinition::Structural(b)) => {
                a.structural_eq(b)
            }
            (TypeDefinition::Nominal(a), TypeDefinition::Nominal(b)) => {
                a.structural_eq(b)
            }
            (TypeDefinition::Union(a), TypeDefinition::Union(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (item_a, item_b) in a.iter().zip(b.iter()) {
                    if !item_a.structural_eq(item_b) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

impl From<ValueContainer> for Type {
    fn from(value: ValueContainer) -> Self {
        Type {
            type_definition: TypeDefinition::Structural(Box::new(value)),
            reference_mutability: None,
        }
    }
}

impl Type {
    /// Creates a nominal type
    /// The mutability is set to None
    pub fn nominal(
        name: &str,
        definition: Reference,
        variant: Option<&str>,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::Nominal(NominalTypeDeclaration {
                name: name.to_string(),
                variant: variant.map(|v| v.to_string()),
                definition: Box::new(definition),
            }),
            reference_mutability: None,
        }
    }

    /// Creates a structural type
    /// The mutability is set to None
    pub fn structural<T>(value: T) -> Self
    where
        T: Into<ValueContainer>,
    {
        Type {
            type_definition: TypeDefinition::Structural(Box::new(value.into())),
            reference_mutability: None,
        }
    }

    /// Creates a union type
    /// The mutability is set to None
    pub fn union(types: Vec<Type>) -> Self {
        Type {
            type_definition: TypeDefinition::Union(types),
            reference_mutability: None,
        }
    }

    pub fn with_mutability(mut self, mutability: ReferenceMutability) -> Self {
        self.reference_mutability = Some(mutability);
        self
    }
}

impl Type {
    pub fn is_structural(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Structural(_))
    }
    pub fn is_nominal(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Nominal(_))
    }
    pub fn is_union(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Union(_))
    }
}

impl Type {
    /// Converts a specific type (e.g. 42u8) to its base type (e.g. integer/u8)
    pub fn get_base_type(&self) -> Type {
        match &self.type_definition {
            TypeDefinition::Structural(value) => value.allowed_type(),
            TypeDefinition::Nominal(_) => self.clone(), // nominal types are already base types
            TypeDefinition::Union(types) => {
                let base_types: Vec<Type> =
                    types.iter().map(|t| t.get_base_type()).collect();
                Type {
                    type_definition: TypeDefinition::Union(base_types),
                    reference_mutability: self.reference_mutability.clone(),
                }
            }
        }
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
        Type::value_matches_type(value, &self)
    }

    /// Matches a value against a type
    pub fn value_matches_type(
        value: &ValueContainer,
        match_type: &Type,
    ) -> bool {
        match match_type {
            // e.g. 1 matches 1 | 2
            Type {
                type_definition: TypeDefinition::Union(types),
                ..
            } => {
                // value must match at least one of the union types
                types.iter().any(|t| Type::value_matches_type(value, t))
            }
            Type {
                type_definition:
                    TypeDefinition::Structural(type_value_container),
                ..
            } => {
                let type_value = type_value_container.as_ref().to_value();
                let type_value = &type_value.borrow().inner;
                match type_value {
                    CoreValue::Type(match_type) => {
                        Type::value_matches_type(value, &match_type)
                    }
                    CoreValue::Array(array) => {
                        todo!("iterate over value")
                    }
                    CoreValue::Tuple(tuple) => {
                        todo!("iterate over value")
                    }
                    CoreValue::Object(object) => {
                        todo!("iterate over value")
                    }
                    // compare primitive values directly for structural equality
                    // fixme: make sure variant matches work as intended (e.g. 1u8 matches 1)
                    // todo: do we need a matchesExact that does not match (e.g. 1u8 matches 1)
                    CoreValue::Integer(_)
                    | CoreValue::Text(_)
                    | CoreValue::Boolean(_)
                    | CoreValue::Decimal(_)
                    | CoreValue::TypedInteger(_)
                    | CoreValue::TypedDecimal(_)
                    | CoreValue::Null
                    | CoreValue::Endpoint(_) => type_value
                        .structural_eq(&value.to_value().borrow().inner),
                }
            }
            Type {
                type_definition:
                    TypeDefinition::Nominal(NominalTypeDeclaration {
                        definition,
                        ..
                    }),
                ..
            } => {
                // compare nominal type directly (e.g. 1 matches integer)
                // TODO: also check if base type matches
                if let TypeDefinition::Nominal(NominalTypeDeclaration {
                    definition: val_type,
                    ..
                }) = value.actual_type().type_definition
                {
                    *val_type.as_ref() == *definition.as_ref()
                } else {
                    false
                }
            }
        }
    }
}

impl CoreValueTrait for Type {}

impl StructuralEq for Type {
    fn structural_eq(&self, other: &Self) -> bool {
        self.type_definition.structural_eq(&other.type_definition)
            && self.reference_mutability == other.reference_mutability
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mutability = self
            .reference_mutability
            .as_ref()
            .map_or("".to_string(), |m| m.to_string());
        write!(f, "{}{}", mutability, self.type_definition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libs::core::create_integer_core_type;
    use datex_core::values::core_values::integer::integer::Integer;

    #[test]
    fn test_match_equal_values() {
        // 1 matches 1
        assert!(Type::value_matches_type(
            &ValueContainer::from(1),
            &Type::structural(1)
        ))
    }

    #[test]
    fn test_match_union() {
        // 1 matches 1 | 2 | 3
        assert!(Type::value_matches_type(
            &ValueContainer::from(Integer::from(1)),
            &Type::union(vec![
                Type::structural(Integer::from(1)),
                Type::structural(Integer::from(2)),
                Type::structural(Integer::from(3)),
            ]),
        ))
    }

    #[test]
    fn test_match_base_type() {
        // 1 matches integer
        let integer = create_integer_core_type(None);
        assert!(Type::value_matches_type(
            &ValueContainer::from(Integer::from(1)),
            &integer
        ))
    }
}
