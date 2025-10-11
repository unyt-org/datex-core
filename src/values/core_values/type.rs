use crate::ast::DatexExpression;
use crate::libs::core::{CoreLibPointerId, get_core_lib_type_reference};
use crate::references::reference::ReferenceMutability;
use crate::references::type_reference::TypeReference;
use crate::traits::structural_eq::StructuralEq;
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::core_value::CoreValue;
use crate::values::core_value_trait::CoreValueTrait;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::text::Text;
use crate::values::value_container::ValueContainer;
use std::cell::RefCell;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

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

impl Type {
    pub fn as_type_container(self) -> TypeContainer {
        TypeContainer::Type(self)
    }
}

impl Type {
    pub const UNIT: Type = Type {
        type_definition: TypeDefinition::Unit,
        base_type: None,
        reference_mutability: None,
    };
    pub fn is_structural(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Structural(_))
    }
    pub fn is_union(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Union(_))
    }
    pub fn is_unit(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Unit)
    }
    pub fn is_reference(&self) -> bool {
        matches!(self.type_definition, TypeDefinition::Reference(_))
    }
    pub fn structural_type(&self) -> Option<&StructuralTypeDefinition> {
        if let TypeDefinition::Structural(s) = &self.type_definition {
            Some(s)
        } else {
            None
        }
    }
}

impl Type {
    /// Creates a new structural type.
    pub fn structural(
        structural_type: impl Into<StructuralTypeDefinition>,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::Structural(structural_type.into()),
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates a new structural list type.
    pub fn list(element_types: Vec<TypeContainer>) -> Self {
        Type {
            type_definition: TypeDefinition::Structural(
                StructuralTypeDefinition::List(element_types),
            ),
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates a new union type.
    pub fn union<T>(types: Vec<T>) -> Self
    where
        T: Into<TypeContainer>,
    {
        let types = types.into_iter().map(|t| t.into()).collect();
        Type {
            type_definition: TypeDefinition::Union(types),
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates a new intersection type.
    pub fn intersection<T>(types: Vec<T>) -> Self
    where
        T: Into<TypeContainer>,
    {
        let types = types.into_iter().map(|t| t.into()).collect();
        Type {
            type_definition: TypeDefinition::Intersection(types),
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates a new reference type.
    pub fn reference(
        reference: impl Into<Rc<RefCell<TypeReference>>>,
        mutability: Option<ReferenceMutability>,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::Reference(reference.into()),
            base_type: None,
            reference_mutability: mutability,
        }
    }

    /// Creates a new function type.
    pub fn function(
        parameters: Vec<(String, TypeContainer)>,
        return_type: impl Into<TypeContainer>,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::Function {
                parameters,
                return_type: Box::new(return_type.into()),
            },
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates a new structural map type.
    pub fn map(
        key_type: impl Into<TypeContainer>,
        value_type: impl Into<TypeContainer>,
    ) -> Self {
        todo!()
        // Type {
        //     type_definition: TypeDefinition::Structural(
        //         StructuralTypeDefinition::Map(Box::new((
        //             key_type.into(),
        //             value_type.into(),
        //         ))),
        //     ),
        //     base_type: None,
        //     reference_mutability: None,
        // }
    }
}

impl Type {
    /// Converts a specific type (e.g. 42u8) to its base nominal type (e.g. integer)
    /// integer/u8 -> integer
    /// integer -> integer
    /// 42u8 -> integer
    /// 42 -> integer
    /// User/variant -> User
    pub fn base_type(&self) -> Option<Rc<RefCell<TypeReference>>> {
        // has direct base type (e.g. integer/u8 -> integer)
        if let Some(base_type) = &self.base_type {
            return Some(base_type.clone());
        }
        // unit type has no base type
        // FIXME
        if self.is_unit() {
            return None;
        }
        Some(match &self.type_definition {
            TypeDefinition::Structural(value) => get_core_lib_type_reference(
                value.get_core_lib_type_pointer_id(),
            ),
            TypeDefinition::Union(_) => {
                todo!("handle union base type"); // generic type base type / type
            }
            TypeDefinition::Reference(reference) => {
                todo!("handle reference base type");
                // return reference.collapse_to_value().borrow()
            }
            _ => panic!("Unhandled type definition for base type"),
        })
    }

    /// 1 matches 1 -> true
    /// 1 matches 2 -> false
    /// 1 matches 1 | 2 -> true
    /// 1 matches "x" | 2 -> false
    /// integer matches 1 | 2 -> false
    pub fn value_matches(&self, value: &ValueContainer) -> bool {
        Type::value_matches_type(value, self)
    }

    /// 1 matches integer -> true
    /// integer matches 1 -> false
    /// integer matches integer -> true
    pub fn matches_type(&self, other: &Type) -> bool {
        // TODO
        println!("Matching types: {} and {}", self, other);

        let other_base_type =
            other.base_type().expect("other type has no base type");
        let other_base_type = other_base_type.borrow();
        let other_base_type = other_base_type.clone().as_type_container();

        match &self.type_definition {
            TypeDefinition::Union(members) => {
                // If self is a union, check if any member matches the other type
                for member in members {
                    if member == &other_base_type {
                        return true;
                    }
                }
                return false;
            }
            TypeDefinition::Intersection(members) => {
                // If self is an intersection, all members must match the other type
                for member in members {
                    if !member.as_type().matches_type(other) {
                        return false;
                    }
                }
                return true;
            }
            // TODO
            _ => {}
        }

        if self.base_type() == other.base_type() {
            return true;
        }
        false
    }
    pub fn matches_reference(&self, other: Rc<RefCell<TypeReference>>) -> bool {
        todo!("implement type reference matching");
        // self.type_matches(&other.type_value)
    }

    /// Matches a value against a type
    pub fn value_matches_type(
        value: &ValueContainer,
        match_type: &Type,
    ) -> bool {
        // if match_type == &value.actual_type().as_type() {
        //     return true;
        // }

        match &match_type.type_definition {
            // e.g. 1 matches 1 | 2
            TypeDefinition::Union(types) => {
                // value must match at least one of the union types
                types
                    .iter()
                    .any(|t| Type::value_matches_type(value, &t.as_type()))
            }
            TypeDefinition::Intersection(types) => {
                // value must match all of the intersection types
                types
                    .iter()
                    .all(|t| Type::value_matches_type(value, &t.as_type()))
            }
            TypeDefinition::Structural(structural_type) => {
                structural_type.value_matches(value)
            }
            TypeDefinition::Reference(reference) => {
                todo!("handle reference type matching");
                //reference.value_matches(value)
            }
            TypeDefinition::Function {
                parameters,
                return_type,
            } => {
                todo!("handle function type matching");
            }
            TypeDefinition::Collection(collection_type) => {
                todo!("handle collection type matching");
            }
            TypeDefinition::Unit => false, // unit type does not match any value
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
        let mutability =
            self.reference_mutability
                .as_ref()
                .map_or("".to_string(), |m| match m {
                    ReferenceMutability::Immutable => "&".to_string(),
                    ReferenceMutability::Mutable => "&mut ".to_string(),
                    ReferenceMutability::Final => "&final ".to_string(),
                });
        let base = self
            .base_type
            .as_ref()
            .map_or("".to_string(), |b| format!(": {}", b.borrow()));
        write!(f, "{}{}{}", mutability, self.type_definition, base)
    }
}

impl From<&CoreValue> for Type {
    fn from(value: &CoreValue) -> Self {
        match value {
            CoreValue::Null => Type::structural(StructuralTypeDefinition::Null),
            CoreValue::Boolean(b) => {
                Type::structural(StructuralTypeDefinition::Boolean(b.clone()))
            }
            CoreValue::Text(s) => Type::structural(s.clone()),
            CoreValue::Decimal(d) => {
                Type::structural(StructuralTypeDefinition::Decimal(d.clone()))
            }
            CoreValue::TypedDecimal(td) => Type::structural(
                StructuralTypeDefinition::TypedDecimal(td.clone()),
            ),
            CoreValue::Integer(i) => {
                Type::structural(StructuralTypeDefinition::Integer(i.clone()))
            }
            CoreValue::TypedInteger(ti) => Type::structural(
                StructuralTypeDefinition::TypedInteger(ti.clone()),
            ),
            CoreValue::Endpoint(e) => {
                Type::structural(StructuralTypeDefinition::Endpoint(e.clone()))
            }
            CoreValue::List(list) => {
                let types = list
                    .iter()
                    .map(|v| Type::from(v.to_value().borrow().inner.clone()))
                    .collect::<Vec<_>>();
                Type::structural(StructuralTypeDefinition::List(
                    types.into_iter().map(TypeContainer::from).collect(),
                ))
            }
            CoreValue::Map(map) => {
                let struct_types = map
                    .into_iter()
                    .map(|(key, value)| {
                        (
                            TypeContainer::from(Type::from(
                                ValueContainer::from(key)
                                    .to_value()
                                    .borrow()
                                    .inner
                                    .clone(),
                            )),
                            TypeContainer::from(Type::from(
                                value.to_value().borrow().inner.clone(),
                            )),
                        )
                    })
                    .collect::<Vec<_>>();
                Type::structural(StructuralTypeDefinition::Map(struct_types))
            }
            e => unimplemented!("Type conversion not implemented for {}", e),
        }
    }
}
impl From<CoreValue> for Type {
    fn from(value: CoreValue) -> Self {
        Type::from(&value)
    }
}

impl TryFrom<&DatexExpression> for StructuralTypeDefinition {
    type Error = ();

    fn try_from(expr: &DatexExpression) -> Result<Self, Self::Error> {
        Ok(match expr {
            DatexExpression::Null => StructuralTypeDefinition::Null,
            DatexExpression::Boolean(b) => {
                StructuralTypeDefinition::Boolean(Boolean::from(*b))
            }
            DatexExpression::Text(s) => {
                StructuralTypeDefinition::Text(Text::from(s.clone()))
            }
            DatexExpression::Decimal(d) => {
                StructuralTypeDefinition::Decimal(d.clone())
            }
            DatexExpression::Integer(i) => {
                StructuralTypeDefinition::Integer(i.clone())
            }
            DatexExpression::Endpoint(e) => {
                StructuralTypeDefinition::Endpoint(e.clone())
            }
            _ => return Err(()),
        })
    }
}

impl TryFrom<&DatexExpression> for Type {
    type Error = ();

    fn try_from(expr: &DatexExpression) -> Result<Self, Self::Error> {
        Ok(Type::structural(StructuralTypeDefinition::try_from(expr)?))
    }
}

#[cfg(test)]
mod tests {
    use crate::values::{
        core_values::{
            integer::{Integer, typed_integer::TypedInteger},
            list::List,
            text::Text,
            r#type::Type,
        },
        value_container::ValueContainer,
    };
    #[test]
    fn test_match_equal_values() {
        // 1u8 matches 1u8
        assert!(Type::value_matches_type(
            &TypedInteger::from(1u8).into(),
            &Type::structural(1u8)
        ));

        // 1u16 matches 1u16
        assert!(Type::value_matches_type(
            &TypedInteger::from(1u16).into(),
            &Type::structural(1u16)
        ));

        // 1 matches 1
        assert!(Type::value_matches_type(
            &ValueContainer::from(Integer::from(1)),
            &Type::structural(Integer::from(1))
        ));

        // "test" matches "test"
        assert!(Type::value_matches_type(
            &ValueContainer::from(Text::from("test")),
            &Type::structural(Text::from("test"))
        ));
    }

    #[test]
    fn test_match_union() {
        // 1 matches (1 | 2 | 3)
        assert!(Type::value_matches_type(
            &ValueContainer::from(Integer::from(1)),
            &Type::union(vec![
                Type::structural(Integer::from(1)),
                Type::structural(Integer::from(2)),
                Type::structural(Integer::from(3)),
            ]),
        ))
    }

    // TODO
    // #[test]
    // fn test_match_combined_type() {
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
