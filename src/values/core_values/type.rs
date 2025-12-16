#[cfg(feature = "compiler")]
use crate::ast::structs::expression::DatexExpressionData;
use crate::libs::core::CoreLibPointerId;
use crate::libs::core::get_core_lib_type;
use crate::libs::core::get_core_lib_type_reference;
use crate::references::reference::ReferenceMutability;
use crate::references::type_reference::TypeReference;
use crate::stdlib::format;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use crate::traits::structural_eq::StructuralEq;
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_value_trait::CoreValueTrait;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::text::Text;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;
use core::fmt::Display;
use core::hash::{Hash, Hasher};
use core::prelude::rust_2024::*;
use core::result::Result;
use core::unimplemented;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    pub type_definition: TypeDefinition,
    pub base_type: Option<Rc<RefCell<TypeReference>>>,
    pub reference_mutability: Option<ReferenceMutability>,
}

// x: &User; Type {reference: }

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
    pub fn unit() -> Self {
        get_core_lib_type(CoreLibPointerId::Unit)
    }
    pub fn null() -> Self {
        get_core_lib_type(CoreLibPointerId::Null)
    }
    pub fn never() -> Self {
        get_core_lib_type(CoreLibPointerId::Never)
    }
    pub fn unknown() -> Self {
        get_core_lib_type(CoreLibPointerId::Unknown)
    }
    pub fn text() -> Self {
        get_core_lib_type(CoreLibPointerId::Text)
    }
    pub fn integer() -> Self {
        get_core_lib_type(CoreLibPointerId::Integer(None))
    }
    pub fn typed_integer(variant: IntegerTypeVariant) -> Self {
        get_core_lib_type(CoreLibPointerId::Integer(Some(variant)))
    }
    pub fn decimal() -> Self {
        get_core_lib_type(CoreLibPointerId::Decimal(None))
    }
    pub fn typed_decimal(variant: DecimalTypeVariant) -> Self {
        get_core_lib_type(CoreLibPointerId::Decimal(Some(variant)))
    }
    pub fn boolean() -> Self {
        get_core_lib_type(CoreLibPointerId::Boolean)
    }
    pub fn endpoint() -> Self {
        get_core_lib_type(CoreLibPointerId::Endpoint)
    }
    pub fn ty() -> Self {
        get_core_lib_type(CoreLibPointerId::Type)
    }
}

impl Type {
    pub const UNIT: Type = Type {
        type_definition: TypeDefinition::Unit,
        base_type: None,
        reference_mutability: None,
    };
    pub fn is_structural(&self) -> bool {
        core::matches!(self.type_definition, TypeDefinition::Structural(_))
    }
    pub fn is_union(&self) -> bool {
        core::matches!(self.type_definition, TypeDefinition::Union(_))
    }
    pub fn is_unit(&self) -> bool {
        core::matches!(self.type_definition, TypeDefinition::Unit)
    }
    pub fn is_reference(&self) -> bool {
        core::matches!(self.type_definition, TypeDefinition::Reference(_))
    }
    pub fn inner_reference(&self) -> Option<Rc<RefCell<TypeReference>>> {
        if let TypeDefinition::Reference(reference) = &self.type_definition {
            Some(reference.clone())
        } else {
            None
        }
    }

    pub fn structural_type_definition(
        &self,
    ) -> Option<&StructuralTypeDefinition> {
        if let TypeDefinition::Structural(s) = &self.type_definition {
            Some(s)
        } else {
            None
        }
    }
    pub fn reference_mutability(&self) -> Option<ReferenceMutability> {
        self.reference_mutability.clone()
    }

    pub fn is_reference_type(&self) -> bool {
        self.reference_mutability.is_some()
    }
}

impl Type {
    /// Creates a new Type with the given TypeDefinition and optional ReferenceMutability
    /// FIXME: If the TypeDefinition is a Reference, the ReferenceMutability must be Some,
    /// otherwise it must be None.
    pub fn new(
        type_definition: TypeDefinition,
        reference_mutability: Option<ReferenceMutability>,
    ) -> Self {
        Type {
            type_definition,
            base_type: None,
            reference_mutability,
        }
    }

    /// Creates a reference type pointing to the given TypeReference with the specified mutability
    pub fn reference(
        type_definition: Rc<RefCell<TypeReference>>,
        reference_mutability: ReferenceMutability,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::Reference(type_definition),
            base_type: None,
            reference_mutability: Some(reference_mutability),
        }
    }

    /// Creates a structural type from the given structural type definition
    pub fn structural(
        structural_type: impl Into<StructuralTypeDefinition>,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::structural(structural_type),
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates a union type from the given member types
    pub fn union<T>(types: Vec<T>) -> Self
    where
        T: Into<Type>,
    {
        Type {
            type_definition: TypeDefinition::union(types),
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates an intersection type from the given member types
    pub fn intersection<T: Into<Type>>(members: Vec<T>) -> Self {
        Type {
            type_definition: TypeDefinition::intersection(members),
            base_type: None,
            reference_mutability: None,
        }
    }

    /// Creates a function type from the given parameter types and return type
    pub fn function(
        parameters: Vec<(String, Type)>,
        return_type: impl Into<Type>,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::function(parameters, return_type),
            base_type: None,
            reference_mutability: None,
        }
    }

    pub fn impl_type(
        base_type: impl Into<Type>,
        impl_types: Vec<PointerAddress>,
    ) -> Self {
        Type {
            type_definition: TypeDefinition::impl_type(base_type, impl_types),
            base_type: None,
            reference_mutability: None,
        }
    }
}

impl Type {
    /// Converts a specific type (e.g. 42u8) to its base nominal type (e.g. integer)
    /// integer/u8 -> integer
    /// integer -> integer
    /// 42u8 -> integer
    /// 42 -> integer
    /// User/variant -> User
    pub fn base_type_reference(&self) -> Option<Rc<RefCell<TypeReference>>> {
        // has direct base type (e.g. integer/u8 -> integer)
        if let Some(base_type) = &self.base_type {
            return Some(base_type.clone());
        }
        // unit type has no base type
        if self.is_unit() {
            return None;
        }
        Some(match &self.type_definition {
            TypeDefinition::Structural(value) => get_core_lib_type_reference(
                value.get_core_lib_type_pointer_id(),
            ),
            TypeDefinition::Union(_) => {
                core::todo!("#322 handle union base type"); // generic type base type / type
            }
            TypeDefinition::Reference(reference) => {
                let type_ref = reference.borrow();
                if let Some(pointer_address) = &type_ref.pointer_address {
                    if let Ok(core_lib_id) =
                        CoreLibPointerId::try_from(pointer_address)
                    {
                        match core_lib_id {
                            // for integer and decimal variants, return the base type
                            CoreLibPointerId::Integer(Some(_)) => {
                                get_core_lib_type_reference(
                                    CoreLibPointerId::Integer(None),
                                )
                            }
                            CoreLibPointerId::Decimal(Some(_)) => {
                                get_core_lib_type_reference(
                                    CoreLibPointerId::Decimal(None),
                                )
                            }
                            // otherwise, reference is already base type
                            _ => reference.clone(),
                        }
                    } else {
                        todo!("handle non-core lib type base type");
                    }
                } else {
                    todo!("handle pointer address none");
                }
            }
            _ => core::panic!("Unhandled type definition for base type"),
        })
    }

    pub fn base_type(&self) -> Option<Type> {
        self.base_type_reference()
            .map(|r| Type::reference(r, ReferenceMutability::Immutable))
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
    /// 1 matches integer | text -> true
    pub fn matches_type(&self, other: &Type) -> bool {
        match &self.type_definition {
            TypeDefinition::Union(members) => {
                // If self is a union, check if any member matches the other type
                for member in members {
                    if member.matches_type(other) {
                        return true;
                    }
                }
                false
            }
            TypeDefinition::Intersection(members) => {
                // If self is an intersection, all members must match the other type
                for member in members {
                    if !member.matches_type(other) {
                        return false;
                    }
                }
                true
            }
            _ => {
                // atomic type match
                Type::atomic_matches_type(self, other)
            }
        }
    }

    /// Checks if an atomic type matches another type
    /// An atomic type can be any type variant besides union or intersection
    pub fn atomic_matches_type(atomic_type: &Type, other: &Type) -> bool {
        // first check if mutability matches
        if atomic_type.reference_mutability != other.reference_mutability {
            return false;
        }

        match &other.type_definition {
            TypeDefinition::Reference(reference) => {
                // compare base type of atomic_type with the referenced type
                if let Some(atomic_base_type_reference) =
                    atomic_type.base_type_reference()
                {
                    *atomic_base_type_reference.borrow() == *reference.borrow()
                } else {
                    false
                }
            }
            TypeDefinition::Union(members) => {
                // atomic type must match at least one member of the union
                for member in members {
                    if Type::atomic_matches_type(atomic_type, member) {
                        return true;
                    }
                }
                false
            }
            TypeDefinition::Intersection(members) => {
                // atomic type must match all members of the intersection
                for member in members {
                    if !Type::atomic_matches_type(atomic_type, member) {
                        return false;
                    }
                }
                true
            }
            _ => {
                // compare type definitions directly
                atomic_type.type_definition == other.type_definition
            }
        }
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
                types.iter().any(|t| Type::value_matches_type(value, t))
            }
            TypeDefinition::Intersection(types) => {
                // value must match all of the intersection types
                types.iter().all(|t| Type::value_matches_type(value, t))
            }
            TypeDefinition::Structural(structural_type) => {
                structural_type.value_matches(value)
            }
            TypeDefinition::Reference(reference) => {
                core::todo!("#327 handle reference type matching");
                //reference.value_matches(value)
            }
            TypeDefinition::Type(inner_type) => {
                // TODO #464: also check mutability of current type?
                inner_type.value_matches(value)
            }
            TypeDefinition::Function {
                parameters,
                return_type,
            } => {
                core::todo!("#328 handle function type matching");
            }
            TypeDefinition::Collection(collection_type) => {
                core::todo!("#329 handle collection type matching");
            }
            TypeDefinition::Unit => false, // unit type does not match any value
            TypeDefinition::Never => false,
            TypeDefinition::Unknown => false,
            TypeDefinition::ImplType(ty, _) => {
                Type::value_matches_type(value, ty)
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mutability =
            self.reference_mutability
                .as_ref()
                .map_or("".to_string(), |m| match m {
                    ReferenceMutability::Immutable => "&".to_string(),
                    ReferenceMutability::Mutable => "&mut ".to_string(),
                });
        let base = self
            .base_type
            .as_ref()
            .map_or("".to_string(), |b| format!(": {}", b.borrow()));
        core::write!(f, "{}{}{}", mutability, self.type_definition, base)
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
                Type::structural(StructuralTypeDefinition::List(types))
            }
            CoreValue::Map(map) => {
                let struct_types = map
                    .into_iter()
                    .map(|(key, value)| {
                        (
                            Type::from(
                                ValueContainer::from(key)
                                    .to_value()
                                    .borrow()
                                    .inner
                                    .clone(),
                            ),
                            Type::from(value.to_value().borrow().inner.clone()),
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

#[cfg(feature = "compiler")]
impl TryFrom<&DatexExpressionData> for StructuralTypeDefinition {
    type Error = ();

    fn try_from(expr: &DatexExpressionData) -> Result<Self, Self::Error> {
        Ok(match expr {
            DatexExpressionData::Null => StructuralTypeDefinition::Null,
            DatexExpressionData::Boolean(b) => {
                StructuralTypeDefinition::Boolean(Boolean::from(*b))
            }
            DatexExpressionData::Text(s) => {
                StructuralTypeDefinition::Text(Text::from(s.clone()))
            }
            DatexExpressionData::Decimal(d) => {
                StructuralTypeDefinition::Decimal(d.clone())
            }
            DatexExpressionData::Integer(i) => {
                StructuralTypeDefinition::Integer(i.clone())
            }
            DatexExpressionData::Endpoint(e) => {
                StructuralTypeDefinition::Endpoint(e.clone())
            }
            _ => return Err(()),
        })
    }
}

#[cfg(feature = "compiler")]
impl TryFrom<&DatexExpressionData> for Type {
    type Error = ();

    fn try_from(expr: &DatexExpressionData) -> Result<Self, Self::Error> {
        Ok(Type::structural(StructuralTypeDefinition::try_from(expr)?))
    }
}

#[cfg(test)]
mod tests {
    use crate::libs::core::{CoreLibPointerId, get_core_lib_type};
    use crate::values::{
        core_values::{
            integer::{Integer, typed_integer::TypedInteger},
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

    #[test]
    fn type_matches_union_type() {
        // 1 matches (1 | 2 | 3)
        assert!(
            Type::structural(Integer::from(1)).matches_type(&Type::union(
                vec![
                    Type::structural(Integer::from(1)),
                    Type::structural(Integer::from(2)),
                    Type::structural(Integer::from(3)),
                ]
            ))
        );

        // 1 matches integer | text
        assert!(
            Type::structural(Integer::from(1)).matches_type(&Type::union(
                vec![
                    get_core_lib_type(CoreLibPointerId::Integer(None)),
                    get_core_lib_type(CoreLibPointerId::Text),
                ]
            ))
        );
    }

    // TODO #330
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
