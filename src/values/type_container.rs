use strum::Display;

use crate::values::core_values::r#type::Type;
use crate::values::type_reference::TypeReference;
use crate::values::value_container::ValueContainer;
use std::cell::RefCell;
use std::fmt::Display;
use std::hash::Hash;
use std::rc::Rc;

// TODO: move match logic and other type stuff here
#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum TypeContainer {
    Type(Type),
    TypeReference(Rc<RefCell<TypeReference>>),
}

impl TypeContainer {
    pub fn as_type(&self) -> Option<Type> {
        match self {
            TypeContainer::Type(t) => Some(t.clone()),
            TypeContainer::TypeReference(tr) => tr.borrow().as_type().cloned(),
        }
    }

    pub fn get_base_type(&self) -> Rc<RefCell<TypeReference>> {
        match self {
            TypeContainer::Type(t) => t.get_base_type(),
            TypeContainer::TypeReference(tr) => tr.clone(),
        }
    }
}

impl Hash for TypeContainer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypeContainer::Type(t) => t.hash(state),
            TypeContainer::TypeReference(tr) => {
                let ptr = Rc::as_ptr(tr);
                ptr.hash(state); // hash the address
            }
        }
    }
}

/**

ValueContainer           <----    TypeContainer

  Value
     Type                <----
     ...
  Reference
     ValueReference
     TypeReference       <-----

*/
impl TypeContainer {
    pub fn value_matches(&self, value: &ValueContainer) -> bool {
        Self::value_matches_type(value, &self)
    }

    /// Matches a value against a type
    pub fn value_matches_type(
        value: &ValueContainer,
        match_type: &Self,
    ) -> bool {
        match match_type {
            TypeContainer::Type(t) => t.value_matches(value),
            TypeContainer::TypeReference(tr) => {
                if let Some(t) = tr.borrow().as_type() {
                    t.value_matches(value)
                } else {
                    false
                }
            }
        }
    }
}
