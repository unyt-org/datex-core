use super::datex_type::CoreValueType;
use crate::values::pointer::Pointer;
use crate::values::traits::identity::Identity;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use std::cell::{Ref, RefCell, RefMut};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use crate::values::core_value::CoreValue;

#[derive(Clone, Debug)]
pub struct Reference(pub Rc<RefCell<ReferenceData>>);

/// Two references are identical if they point to the same data
impl Identity for Reference {
    fn identical(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Reference {}

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
            .resolve_current_value()
            .borrow()
            .structural_eq(&other.borrow().resolve_current_value().borrow())
    }
}

impl ValueEq for Reference {
    fn value_eq(&self, other: &Self) -> bool {
        // Two references are value-equal if their current resolved values are equal
        self.borrow()
            .resolve_current_value()
            .borrow()
            .value_eq(&other.borrow().resolve_current_value().borrow())
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
        Reference::new_from_value_container(value_container, allowed_type)
    }
}

// Implement Deref to allow access to ReferenceData directly
impl Deref for Reference {
    type Target = RefCell<ReferenceData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Reference {
    pub fn new_from_value_container(
        value_container: ValueContainer,
        allowed_type: CoreValueType,
    ) -> Self {
        let reference = Reference(Rc::new(RefCell::new(ReferenceData {
            value_container,
            pointer: None,
            allowed_type,
            parents: Vec::new(),
        })));
        reference.upgrade_inner_combined_values_to_references();
        reference
    }

    /// Collapses the reference chain to most inner reference to which this reference points.
    pub fn collapse_reference_chain(&self) -> Reference {
        match &self.borrow().value_container {
            ValueContainer::Reference(reference) => {
                // If this is a reference, resolve it to its current value
                reference.collapse_reference_chain()
            }
            ValueContainer::Value(_) => {
                // If this is a value, return it directly
                self.clone()
            }
        }
    }

    /// Runs a closure with the current value of this reference.
    pub fn with_value<R, F: FnOnce(&mut Value) -> R>(&self, f: F) -> R {
        let reference = self.collapse_reference_chain();
        let mut ref_value = reference.borrow_mut();
        match &mut ref_value.value_container {
            ValueContainer::Value(value) => {
                f(value)
            }
            ValueContainer::Reference(_) => {
                unreachable!("Expected a ValueContainer::Value, but found a Reference")
            }
        }
    }

    /// Registers a parent for this reference.
    pub fn add_parent(&self, parent: &Reference) {
        self.borrow_mut().parents.push(Rc::downgrade(&parent.0));
    }


    /// Removes a parent from this reference.
    pub fn remove_parent(&self, parent: &Reference) {
        self.borrow_mut().parents.retain(|p| {
            if let Some(p) = p.upgrade() {
                !Rc::ptr_eq(&p, &parent.0)
            } else {
                true // keep weak references that are no longer valid
            }
        });
    }

    /// Sets a text property on the value if applicable (e.g. for objects)
    pub fn try_set_text_property(
        &self,
        key: &str,
        mut val: ValueContainer,
    ) -> Result<(), String> {

        // Ensure the value is a reference if it is a combined value (e.g. an object)
        val = val.upgrade_combined_value_to_reference();

        self.with_value(|value| {
            match value.inner {
                CoreValue::Object(ref mut obj) => {
                    // If the value is an object, set the property
                    obj.set(key, self.bind_child(val));
                }
                _ => {
                    // If the value is not an object, we cannot set a property
                    return Err(format!(
                        "Cannot set property '{}' on non-object value: {:?}",
                        key, value
                    ));
                }
            }
            Ok(())
        })
    }

    pub fn try_get_text_property(&self, key: &str) -> Result<Option<ValueContainer>, String> {
        self.with_value(|value | {
            match value.inner {
                CoreValue::Object(ref mut obj) => {
                    // If the value is an object, get the property
                    Ok(obj.try_get(key).cloned())
                }
                _ => {
                    // If the value is not an object, we cannot get a property
                    Err(format!(
                        "Cannot get property '{}' on non-object value: {:?}",
                        key, value
                    ))
                }
            }
        })
    }

    /// upgrades all inner combined values (e.g. object properties) to references
    pub fn upgrade_inner_combined_values_to_references(
        &self,
    ) {
        self.with_value(|value| {
            match &mut value.inner {
                CoreValue::Object(obj) => {
                    // Iterate over all properties and upgrade them to references
                    for (_, prop) in obj.iter_mut() {
                        // TODO: no clone here, implement some sort of map
                        *prop = self.bind_child(prop.clone());
                    }
                },
                // TODO: other combined value types should be added here
                _ => {
                    // If the value is not an object, we do not need to upgrade anything
                }
            }
        });
    }

    /// Binds a child value to this reference, ensuring the child is a reference if it is a combined value
    /// and adding self to the parent list of the child.
    pub fn bind_child(&self, child: ValueContainer) -> ValueContainer {
        // Ensure the child is a reference if it is a combined value
        let child = child.upgrade_combined_value_to_reference();

        // Add the child as a parent of this reference
        child.with_maybe_reference(|child_ref| {
            child_ref.add_parent(self);
        });

        child
    }
}


#[derive(Debug)]
pub struct ReferenceData {
    /// the value that this reference points to
    pub value_container: ValueContainer,
    /// pointer information
    /// this can be None if only a local reference is needed
    pointer: Option<Pointer>,
    /// custom type for the pointer that the Datex value is allowed to reference
    pub allowed_type: CoreValueType,
    /// weak refs to all parents of this reference for update propagation
    pub parents: Vec<Weak<RefCell<ReferenceData>>>,
}

impl PartialEq for ReferenceData {
    fn eq(&self, other: &Self) -> bool {
        // Two ReferenceData are equal if their value containers are equal
        self.value_container.value_eq(&other.value_container)
    }
}

impl ReferenceData {
    pub fn pointer_id(&self) -> Option<u64> {
        self.pointer.as_ref().map(|p| p.pointer_id())
    }

    pub fn current_value_container(&self) -> &ValueContainer {
        &self.value_container
    }

    pub fn resolve_current_value(&self) -> Rc<RefCell<Value>> {
        self.value_container.to_value()
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use datex_core::values::core_values::object::Object;
    use super::*;
    use crate::values::traits::value_eq::ValueEq;
    use crate::{assert_identical, assert_structural_eq, assert_value_eq};

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

    #[test]
    fn test_nested_references() {
        let mut object_a = Object::new();
        object_a.set("number", ValueContainer::from(42));
        object_a.set("obj", ValueContainer::from(Object::new()));

        // construct object_a as a value first
        let object_a_val = ValueContainer::new_value(object_a);

        // create object_b as a reference
        let object_b_ref = ValueContainer::new_reference(Object::new());

        // set object_a as property of b. This should create a reference to a clone of object_a that
        // is upgraded to a reference
        object_b_ref
            .with_maybe_reference(|b_ref| {
                b_ref.try_set_text_property("a", object_a_val.clone())
            });

        println!("Object B Reference: {:#?}", object_b_ref);

        // assert that the reference to object_a is set correctly
        object_b_ref
            .with_maybe_reference(|b_ref| {
                let object_a_ref = b_ref.try_get_text_property("a").unwrap().unwrap();
                assert_structural_eq!(object_a_ref, object_a_val);
                // object_a_ref should be a reference
                assert_matches!(object_a_ref, ValueContainer::Reference(_));
                object_a_ref.with_maybe_reference(|a_ref| {
                    // object_a_ref.number should be a value
                    assert_matches!(a_ref.try_get_text_property("number"), Ok(Some(ValueContainer::Value(_))));
                    // object_a_ref.obj should be a reference
                    assert_matches!(a_ref.try_get_text_property("obj"), Ok(Some(ValueContainer::Reference(_))));
                });
            })
            .expect("object_b_ref should be a reference");

        // assert that parents are set correctly
        object_b_ref
            .with_maybe_reference(|b_ref| {
                // b has no parents
                {
                    let parents = &b_ref.borrow().parents;
                    assert_eq!(parents.len(), 0, "Object B should not have any parents");
                }

                // a has one parent, which is b
                let object_a_ref = b_ref.try_get_text_property("a").unwrap().unwrap();

                object_a_ref
                    .with_maybe_reference(|a_ref| {
                        // a_ref should have one parent, which is b_ref
                        {
                            let parents = &a_ref.borrow().parents;
                            // object_b should be a parent of object_a
                            assert_eq!(parents.len(), 1, "Object A should have one parent");
                            let parent = parents.first().unwrap();
                            // parent should be a weak reference to object_b
                            assert!(parent.upgrade().is_some(), "Parent should be a valid reference");
                            let parent_ref = parent.upgrade().unwrap();
                            // parent reference should be equal to object_b_ref
                            assert_eq!(parent_ref, b_ref.0, "Parent reference should be equal to object_b_ref");
                        }

                        // object_a_ref.obj should have object_a as a parent
                        let object_a_obj_ref = a_ref.try_get_text_property("obj").unwrap().unwrap();
                        object_a_obj_ref
                            .with_maybe_reference(|obj_ref| {
                                let obj_parents = &obj_ref.borrow().parents;
                                assert_eq!(obj_parents.len(), 1, "Object A's obj should have one parent");
                                let obj_parent = obj_parents.first().unwrap();
                                // parent should be a weak reference to object_a
                                assert!(obj_parent.upgrade().is_some(), "Parent should be a valid reference");
                                let obj_parent_ref = obj_parent.upgrade().unwrap();
                                // parent reference should be equal to object_a_ref
                                assert_eq!(obj_parent_ref, a_ref.0, "Parent reference should be equal to object_a_ref");
                            })
                            .expect("object_a_obj_ref should be a reference");
                    })
                    .expect("object_a_ref should be a reference");

            })
            .expect("object_b_ref should be a reference");
    }
}
