use std::cell::RefCell;
use crate::values::core_value::CoreValue;
use crate::values::core_value_trait::CoreValueTrait;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::ValueContainer;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use datex_core::values::core_values::endpoint::Endpoint;
use datex_core::values::core_values::integer::integer::Integer;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::text::Text;
use crate::values::reference::{Reference, ReferenceMutability};
use crate::values::type_reference::TypeReference;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    pub type_definition: TypeDefinition,
    pub base_type: Option<Rc<RefCell<TypeReference>>>,
    pub reference_mutability: Option<ReferenceMutability>,
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_definition.hash(state);
        self.reference_mutability.hash(state);
        if let Some(ptr) = &self.base_type {
            let ptr = Rc::as_ptr(ptr);
            ptr.hash(state); // hash the address
        }
    }
}


// type integer2 = integer; <- $0101010|"integer2"

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TypeDefinition {
    // {x: integer, y: text}
    Structural(StructuralType),
    Reference(Box<Reference>),

    // e.g. A | B | C
    Union(Vec<Type>),
    // ()
    Unit,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum StructuralType {
    Integer(Integer),
    TypedInteger(TypedInteger),
    Decimal(Decimal),
    TypedDecimal(TypedDecimal),
    Text(Text),
    Boolean(Boolean),
    Endpoint(Endpoint),
    Null,
    Array(Vec<Type>),
    Tuple(Vec<(Type, Type)>),
    Object(Vec<(Type, Type)>),
}

impl Display for StructuralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructuralType::Integer(integer) => write!(f, "{}", integer),
            StructuralType::TypedInteger(typed_integer) => write!(f, "{}", typed_integer),
            StructuralType::Decimal(decimal) => write!(f, "{}", decimal),
            StructuralType::TypedDecimal(typed_decimal) => write!(f, "{}", typed_decimal),
            StructuralType::Text(text) => write!(f, "{}", text),
            StructuralType::Boolean(boolean) => write!(f, "{}", boolean),
            StructuralType::Endpoint(endpoint) => write!(f, "{}", endpoint),
            StructuralType::Null => write!(f, "null"),
            StructuralType::Array(types) => {
                let types_str: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "[{}]", types_str.join(", "))
            }
            StructuralType::Tuple(elements) => {
                let elements_str: Vec<String> = elements
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "({})", elements_str.join(", "))
            }
            StructuralType::Object(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", fields_str.join(", "))
            }
        }
    }
}

impl Display for TypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeDefinition::Structural(value) => write!(f, "{}", value),
            TypeDefinition::Reference(reference) => write!(f, "{:?}", reference),
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

impl Type {
    pub fn is_structural(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Structural(_))
    }
    pub fn is_union(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Union(_))
    }
}

impl Type {
    /// Converts a specific type (e.g. 42u8) to its base type (e.g. integer/u8)
    pub fn get_base_type(&self) -> Rc<RefCell<TypeReference>> {
        // has direct base type (e.g. integer/u8 -> integer)
        if let Some(base_type) = &self.base_type {
            return base_type.clone();
        }
        match &self.type_definition {
            TypeDefinition::Structural(value) => {
                
            }
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
            Type {
                type_definition: TypeDefinition::Base,
                ..
            } => true,
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

    // #[test]
    // fn test_match_base_type() {
    //     // 1 matches integer
    //     let integer = create_integer_core_type(None);
    //     assert!(Type::value_matches_type(
    //         &ValueContainer::from(Integer::from(1)),
    //         &integer
    //     ))
    // }
}
