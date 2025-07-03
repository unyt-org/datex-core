use super::datex_type::CoreValueType;
use crate::values::pointer::Pointer;
use crate::values::traits::identity::Identity;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Debug, Eq)]
pub struct Reference(pub Rc<RefCell<ReferenceData>>);

/// Two references are identical if they point to the same data
impl Identity for Reference {
    fn identical(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// PartialEq corresponds to pointer equality / identity for `Reference`.
impl PartialEq for Reference {
    fn eq(&self, other: &Self) -> bool {
        self.identical(other)
    }
}

impl StructuralEq for Reference {
    fn structural_eq(&self, other: &Self) -> bool {
        // Two references are structurally equal if their current resolved values are equal
        self.borrow()
            .current_resolved_value()
            .borrow()
            .structural_eq(&other.borrow().current_resolved_value().borrow())
    }
}

impl ValueEq for Reference {
    fn value_eq(&self, other: &Self) -> bool {
        // Two references are value-equal if their current resolved values are equal
        self.borrow()
            .current_resolved_value()
            .borrow()
            .value_eq(&other.borrow().current_resolved_value().borrow())
    }
}

impl Hash for Reference {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Rc::as_ptr(&self.0); // gets *const RefCell<ReferenceData>
        ptr.hash(state); // hash the address
    }
}

impl<T: Into<ValueContainer>> From<T> for Reference {
    fn from(value_container: T) -> Self {
        let value_container = value_container.into();
        let allowed_type =
            value_container.to_value().borrow().actual_type.clone();
        Reference(Rc::new(RefCell::new(ReferenceData {
            value_container,
            pointer: None,
            allowed_type,
        })))
    }
}

// Implement Deref to allow access to ReferenceData directly
impl Deref for Reference {
    type Target = RefCell<ReferenceData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ReferenceData {
    /// the value that this reference points to
    pub value_container: ValueContainer,
    /// pointer information
    /// this can be None if only a local reference is needed
    pointer: Option<Pointer>,
    /// custom type for the pointer that the Datex value is allowed to reference
    pub allowed_type: CoreValueType,
}

impl ReferenceData {
    pub fn pointer_id(&self) -> Option<u64> {
        self.pointer.as_ref().map(|p| p.pointer_id())
    }

    pub fn current_value_container(&self) -> &ValueContainer {
        &self.value_container
    }

    pub fn current_resolved_value(&self) -> Rc<RefCell<Value>> {
        self.value_container.to_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::traits::value_eq::ValueEq;
    use crate::{assert_identical, assert_value_eq};

    #[test]
    fn test_reference_identity() {
        let value = 42;
        let reference1 = Reference::from(value);
        let reference2 = reference1.clone();

        // cloned reference should be equal (identical)
        assert_eq!(reference1, reference2);
        // value containers containing the references should also be equal
        assert_eq!(
            ValueContainer::Reference(reference1.clone()),
            ValueContainer::Reference(reference2.clone())
        );
        // assert_identical! should also confirm identity
        assert_identical!(reference1.clone(), reference2);
        // separate reference containing the same value should not be equal
        assert_ne!(reference1, Reference::from(value));
    }

    #[test]
    fn test_reference_value_equality() {
        let value = 42;
        let reference1 = ValueContainer::Reference(Reference::from(value));
        let reference2 = ValueContainer::Reference(Reference::from(value));

        // different references should not be equal a.k.a. identical
        assert_ne!(reference1, reference2);
        // but their current resolved values should be equal
        assert_value_eq!(reference1, ValueContainer::from(value));
    }

    #[test]
    fn test_reference_structural_equality() {
        let reference1 = Reference::from(42.0);
        let reference2 = Reference::from(42);

        // different references should not be equal a.k.a. identical
        assert_ne!(reference1, reference2);
        // but their current resolved values should be structurally equal
        assert!(!reference1.structural_eq(&reference2));
    }
}
