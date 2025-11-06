use crate::libs::core::{CoreLibPointerId, get_core_lib_type};
use crate::references::type_reference::TypeReference;
use crate::stdlib::rc::Rc;
use crate::traits::structural_eq::StructuralEq;
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::r#type::Type;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;
use core::fmt::Display;
use core::hash::Hash;
use core::prelude::rust_2024::*;

// TODO #376: move match logic and other type stuff here
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeContainer {
    Type(Type),
    TypeReference(Rc<RefCell<TypeReference>>),
}

impl Display for TypeContainer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TypeContainer::Type(t) => core::write!(f, "{}", t),
            TypeContainer::TypeReference(tr) => {
                let tr = tr.borrow();
                core::write!(f, "{}", tr)
            }
        }
    }
}

impl From<Type> for TypeContainer {
    fn from(value: Type) -> Self {
        TypeContainer::Type(value)
    }
}
impl From<Rc<RefCell<TypeReference>>> for TypeContainer {
    fn from(value: Rc<RefCell<TypeReference>>) -> Self {
        TypeContainer::TypeReference(value)
    }
}
impl From<TypeReference> for TypeContainer {
    fn from(value: TypeReference) -> Self {
        TypeContainer::TypeReference(Rc::new(RefCell::new(value)))
    }
}

impl TypeContainer {
    pub fn as_type(&self) -> Type {
        match self {
            TypeContainer::Type(t) => t.clone(),
            TypeContainer::TypeReference(tr) => tr.borrow().as_type().clone(),
        }
    }

    pub fn base_type(&self) -> TypeContainer {
        match self {
            TypeContainer::Type(t) => {
                if let Some(base) = t.base_type() {
                    TypeContainer::TypeReference(base)
                } else {
                    TypeContainer::Type(t.clone())
                }
            }
            TypeContainer::TypeReference(tr) => {
                if let Some(base) = tr.borrow().base_type() {
                    TypeContainer::TypeReference(base)
                } else {
                    TypeContainer::TypeReference(tr.clone())
                }
            }
        }
    }
}

impl Hash for TypeContainer {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypeContainer::Type(t) => t.hash(state),
            TypeContainer::TypeReference(tr) => {
                let ptr = Rc::as_ptr(tr);
                ptr.hash(state); // hash the address
            }
        }
    }
}

impl StructuralEq for TypeContainer {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeContainer::Type(a), TypeContainer::Type(b)) => {
                a.structural_eq(b)
            }
            (
                TypeContainer::TypeReference(a),
                TypeContainer::TypeReference(b),
            ) => a.borrow().as_type().structural_eq(b.borrow().as_type()),
            _ => false,
        }
    }
}

impl TypeContainer {
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
    pub fn r#type() -> Self {
        get_core_lib_type(CoreLibPointerId::Type)
    }
}

impl TypeContainer {
    pub fn value_matches(&self, value: &ValueContainer) -> bool {
        Self::value_matches_type(value, self)
    }

    /// Matches a value against a type
    pub fn value_matches_type(
        value: &ValueContainer,
        match_type: &Self,
    ) -> bool {
        match match_type {
            TypeContainer::Type(t) => t.value_matches(value),
            TypeContainer::TypeReference(tr) => {
                tr.borrow().as_type().value_matches(value)
            }
        }
    }

    /// Matches if one type matches the other
    pub fn matches_type(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeContainer::Type(a), TypeContainer::Type(b)) => {
                a.matches_type(b)
            }
            (
                TypeContainer::TypeReference(a),
                TypeContainer::TypeReference(b),
            ) => a.borrow().matches_reference(b.clone()),
            (TypeContainer::TypeReference(a), TypeContainer::Type(b)) => {
                a.borrow().matches_type(b)
            }
            (TypeContainer::Type(a), TypeContainer::TypeReference(b)) => {
                a.matches_reference(b.clone())
            }
        }
    }
}
