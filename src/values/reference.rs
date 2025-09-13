use crate::dif::{DIFUpdate, DIFValue};
use crate::values::core_value::CoreValue;
use crate::values::core_values::r#type::Type;
use crate::values::core_values::r#type::definition::TypeDefinition;

use crate::values::pointer::PointerAddress;
use crate::values::traits::identity::Identity;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::type_container::TypeContainer;
use crate::values::type_reference::{NominalTypeDeclaration, TypeReference};
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use crate::values::value_reference::ValueReference;
use std::cell::RefCell;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug)]
pub enum ObserveError {
    ImmutableReference,
}

#[derive(Debug)]
pub enum AccessError {
    ImmutableReference,
    InvalidOperation(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceMutability {
    Mutable,
    Immutable,
}

impl Display for ReferenceMutability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceMutability::Mutable => write!(f, "&mut"),
            ReferenceMutability::Immutable => write!(f, "&"),
        }
    }
}

/*

x = x.try_set_property();

*/

#[derive(Debug, Clone)]
pub enum Reference {
    ValueReference(Rc<RefCell<ValueReference>>),
    TypeReference(Rc<RefCell<TypeReference>>),
}

impl From<ValueReference> for Reference {
    fn from(reference: ValueReference) -> Self {
        Reference::ValueReference(Rc::new(RefCell::new(reference)))
    }
}
impl From<TypeReference> for Reference {
    fn from(reference: TypeReference) -> Self {
        Reference::TypeReference(Rc::new(RefCell::new(reference)))
    }
}

/// Two references are identical if they point to the same data
impl Identity for Reference {
    fn identical(&self, other: &Self) -> bool {
        match (self, other) {
            (Reference::ValueReference(a), Reference::ValueReference(b)) => {
                Rc::ptr_eq(a, b)
            }
            (Reference::TypeReference(a), Reference::TypeReference(b)) => {
                Rc::ptr_eq(a, b)
            }
            _ => false,
        }
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
        match (self, other) {
            (Reference::TypeReference(a), Reference::TypeReference(b)) => {
                a.borrow().type_value.structural_eq(&b.borrow().type_value)
            }
            (Reference::ValueReference(a), Reference::ValueReference(b)) => a
                .borrow()
                .value_container
                .structural_eq(&b.borrow().value_container),
            _ => false,
        }
    }
}

impl ValueEq for Reference {
    fn value_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Reference::TypeReference(a), Reference::TypeReference(b)) => {
                a.borrow().type_value.structural_eq(&b.borrow().type_value)
            }
            (Reference::ValueReference(a), Reference::ValueReference(b)) => a
                .borrow()
                .value_container
                .value_eq(&b.borrow().value_container),
            _ => false,
        }
    }
}

impl Hash for Reference {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Reference::TypeReference(tr) => {
                let ptr = Rc::as_ptr(tr);
                ptr.hash(state); // hash the address
            }
            Reference::ValueReference(vr) => {
                let ptr = Rc::as_ptr(vr);
                ptr.hash(state); // hash the address
            }
        }
    }
}

impl<T: Into<ValueContainer>> From<T> for Reference {
    /// Creates a new immutable reference from a value container.
    fn from(value_container: T) -> Self {
        let value_container = value_container.into();
        Reference::try_new_from_value_container(
            value_container,
            None,
            None,
            ReferenceMutability::Immutable,
        )
        .unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReferenceFromValueContainerError {
    InvalidType,
    MutableTypeReference,
}

impl Reference {
    pub fn pointer_address(&self) -> Option<PointerAddress> {
        match self {
            Reference::ValueReference(vr) => {
                vr.borrow().pointer_address().clone()
            }
            Reference::TypeReference(tr) => tr.borrow().pointer_address.clone(),
        }
    }

    pub fn set_pointer_address(&self, pointer_address: PointerAddress) {
        if self.pointer_address().is_some() {
            panic!(
                "Cannot set pointer address on reference that already has one"
            );
        }
        match self {
            Reference::ValueReference(vr) => {
                vr.borrow_mut().pointer_address = Some(pointer_address)
            }
            Reference::TypeReference(tr) => {
                tr.borrow_mut().pointer_address = Some(pointer_address)
            }
        }
    }

    pub(crate) fn mutability(&self) -> ReferenceMutability {
        match self {
            Reference::ValueReference(vr) => vr.borrow().mutability.clone(),
            Reference::TypeReference(_) => ReferenceMutability::Immutable,
        }
    }

    /// Creates a new reference from a value container
    pub fn try_new_from_value_container(
        value_container: ValueContainer,
        allowed_type: Option<TypeContainer>,
        maybe_pointer_id: Option<PointerAddress>,
        mutability: ReferenceMutability,
    ) -> Result<Self, ReferenceFromValueContainerError> {
        Ok(match value_container {
            ValueContainer::Reference(ref reference) => {
                let allowed_type =
                    allowed_type.unwrap_or_else(|| reference.allowed_type());
                // TODO: make sure allowed type is superset of reference's allowed type
                Reference::ValueReference(Rc::new(RefCell::new(
                    ValueReference {
                        value_container,
                        pointer_address: maybe_pointer_id,
                        allowed_type,
                        observers: Vec::new(),
                        mutability,
                    },
                )))
            }
            ValueContainer::Value(value) => {
                match value.inner {
                    // create TypeReference if the value is a Type
                    CoreValue::Type(type_value) => {
                        // TODO: allowed_type "Type" is also allowed
                        if allowed_type.is_some() {
                            return Err(
                                ReferenceFromValueContainerError::InvalidType,
                            );
                        }
                        if mutability != ReferenceMutability::Immutable {
                            return Err(ReferenceFromValueContainerError::MutableTypeReference);
                        }
                        Reference::new_from_type(
                            type_value,
                            maybe_pointer_id,
                            None,
                        )
                    }
                    // otherwise create ValueReference
                    _ => {
                        let allowed_type = allowed_type.unwrap_or_else(|| {
                            value.actual_type.as_ref().clone()
                        });
                        Reference::ValueReference(Rc::new(RefCell::new(
                            ValueReference {
                                value_container: ValueContainer::Value(value),
                                pointer_address: maybe_pointer_id,
                                allowed_type,
                                observers: Vec::new(),
                                mutability,
                            },
                        )))
                    }
                }
            }
        })
    }

    pub fn new_from_type(
        type_value: Type,
        maybe_pointer_address: Option<PointerAddress>,
        maybe_nominal_type_declaration: Option<NominalTypeDeclaration>,
    ) -> Self {
        let type_reference = TypeReference {
            pointer_address: maybe_pointer_address,
            nominal_type_declaration: maybe_nominal_type_declaration,
            type_value,
        };
        Reference::TypeReference(Rc::new(RefCell::new(type_reference)))
    }

    pub fn try_mut_from(
        value_container: ValueContainer,
    ) -> Result<Self, ReferenceFromValueContainerError> {
        Reference::try_new_from_value_container(
            value_container,
            None,
            None,
            ReferenceMutability::Mutable,
        )
    }

    /// Collapses the reference chain to most inner reference to which this reference points.
    pub fn collapse_reference_chain(&self) -> Reference {
        match self {
            Reference::TypeReference(tr) => {
                match &tr.borrow().type_value.type_definition {
                    TypeDefinition::Reference(reference) => {
                        // If this is a reference type, resolve it to its current reference
                        reference.collapse_reference_chain()
                    }
                    _ => {
                        // If this is not a reference type, return it directly
                        self.clone()
                    }
                }
            }
            Reference::ValueReference(vr) => {
                match &vr.borrow().value_container {
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
        }
    }

    /// Converts a reference to its current value, collapsing any reference chains and converting type references to type values.
    pub fn collapse_to_value(&self) -> Rc<RefCell<Value>> {
        let reference = self.collapse_reference_chain();
        match reference {
            Reference::ValueReference(vr) => match &vr.borrow().value_container
            {
                ValueContainer::Value(_) => {
                    vr.borrow().value_container.to_value()
                }
                ValueContainer::Reference(_) => unreachable!(
                    "Expected a ValueContainer::Value, but found a Reference"
                ),
            },
            // TODO: can we optimize this to avoid cloning the type value?
            Reference::TypeReference(tr) => Rc::new(RefCell::new(Value::from(
                CoreValue::Type(tr.borrow().type_value.clone()),
            ))),
        }
    }

    /// Runs a closure with the current value of this reference.
    pub fn with_value<R, F: FnOnce(&mut Value) -> R>(&self, f: F) -> Option<R> {
        let reference = self.collapse_reference_chain();

        match reference {
            Reference::ValueReference(vr) => {
                match &mut vr.borrow_mut().value_container {
                    ValueContainer::Value(value) => Some(f(value)),
                    ValueContainer::Reference(_) => {
                        unreachable!(
                            "Expected a ValueContainer::Value, but found a Reference"
                        )
                    }
                }
            }
            Reference::TypeReference(_) => None,
        }
    }

    /// Sets a text property on the value if applicable (e.g. for objects)
    pub fn try_set_text_property(
        &self,
        key: &str,
        mut val: ValueContainer,
    ) -> Result<(), AccessError> {
        // Ensure the value is a reference if it is a combined value (e.g. an object)
        val = val.upgrade_combined_value_to_reference();

        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut obj) => {
                    // If the value is an object, set the property
                    obj.set(key, self.bind_child(val));
                }
                _ => {
                    // If the value is not an object, we cannot set a property
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot set property '{}' on non-object value: {:?}",
                        key, value
                    )));
                }
            }
            Ok(())
        })
        .unwrap_or(Err(AccessError::ImmutableReference))
    }

    pub fn try_get_value_for_key<T: Into<ValueContainer>>(
        &self,
        key: T,
    ) -> Result<Option<ValueContainer>, AccessError> {
        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut map) => {
                    // If the value is an object, get the property
                    Ok(map.get(&key.into()).cloned())
                }
                _ => {
                    // If the value is not an object, we cannot get a property
                    Err(AccessError::InvalidOperation(
                        "Cannot get property".to_string(),
                    ))
                }
            }
        })
        .expect("todo: implement property access for types")
    }

    pub fn try_set_value<T: Into<ValueContainer>>(
        &self,
        value: T,
    ) -> Result<(), String> {
        // TODO: ensure type compatibility with allowed_type
        let value_container = &value.into();
        self.with_value(|core_value| {
            // Set the value directly, ensuring it is a ValueContainer
            core_value.inner =
                value_container.to_value().borrow().inner.clone();
        });

        // Notify observers of the update
        if self.has_observers() {
            let dif = DIFUpdate::Replace(DIFValue::from(value_container));
            self.notify_observers(&dif);
        }

        Ok(())
    }

    /// upgrades all inner combined values (e.g. object properties) to references
    pub fn upgrade_inner_combined_values_to_references(&self) {
        self.with_value(|value| {
            match &mut value.inner {
                CoreValue::Map(map) => {
                    // Iterate over all properties and upgrade them to references
                    for (_, prop) in map.iter_mut() {
                        // TODO: no clone here, implement some sort of map
                        *prop = self.bind_child(prop.clone());
                    }
                }
                // TODO: other combined value types should be added here
                _ => {
                    // If the value is not an object, we do not need to upgrade anything
                }
            }
        });
    }

    /// Binds a child value to this reference, ensuring the child is a reference if it is a combined value
    fn bind_child(&self, child: ValueContainer) -> ValueContainer {
        // Ensure the child is a reference if it is a combined value

        child.upgrade_combined_value_to_reference()
    }

    /// Adds an observer to this reference that will be notified on value changes.
    /// Returns an error if the reference is immutable
    pub fn observe<F: Fn(&DIFUpdate) + 'static>(
        &self,
        observer: F,
    ) -> Result<(), ObserveError> {
        // Add the observer to the list of observers
        match self {
            Reference::TypeReference(_) => {
                // Type references do not have observers
                Err(ObserveError::ImmutableReference)
            }
            Reference::ValueReference(vr) => {
                vr.borrow_mut().observers.push(Box::new(observer));
                Ok(())
            }
        }
        // TODO: also set observers on child references if not yet active, keep track of active observers
    }

    fn notify_observers(&self, dif: &DIFUpdate) {
        match self {
            Reference::TypeReference(_) => {
                // Type references do not have observers
            }
            Reference::ValueReference(vr) => {
                /// Notify all observers of the update
                for observer in &vr.borrow().observers {
                    observer(dif);
                }
            }
        }
    }

    fn has_observers(&self) -> bool {
        // Check if there are any observers registered
        match self {
            Reference::TypeReference(_) => false,
            Reference::ValueReference(vr) => !vr.borrow().observers.is_empty(),
        }
    }

    pub fn allowed_type(&self) -> TypeContainer {
        match self {
            Reference::ValueReference(vr) => vr.borrow().allowed_type.clone(),
            Reference::TypeReference(_) => todo!("type Type"),
        }
    }

    pub fn actual_type(&self) -> TypeContainer {
        match self {
            Reference::ValueReference(vr) => vr
                .borrow()
                .value_container
                .to_value()
                .borrow()
                .actual_type()
                .clone(),
            Reference::TypeReference(tr) => todo!("type Type"),
        }
    }

    pub fn is_type(&self) -> bool {
        match self {
            Reference::TypeReference(_) => true,
            Reference::ValueReference(vr) => {
                vr.borrow().resolve_current_value().borrow().is_type()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::DatexExpression::Ref;
    use crate::values::traits::value_eq::ValueEq;
    use crate::{assert_identical, assert_structural_eq, assert_value_eq};
    use datex_core::values::core_values::map::Map;
    use std::assert_matches::assert_matches;

    #[test]
    fn reference_identity() {
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
    fn reference_value_equality() {
        let value = 42;
        let reference1 = ValueContainer::Reference(Reference::from(value));
        let reference2 = ValueContainer::Reference(Reference::from(value));

        // different references should not be equal a.k.a. identical
        assert_ne!(reference1, reference2);
        // but their current resolved values should be equal
        assert_value_eq!(reference1, ValueContainer::from(value));
    }

    #[test]
    fn reference_structural_equality() {
        let reference1 = Reference::from(42.0);
        let reference2 = Reference::from(42);

        // different references should not be equal a.k.a. identical
        assert_ne!(reference1, reference2);
        // but their current resolved values should be structurally equal
        assert!(!reference1.structural_eq(&reference2));
    }

    #[test]
    fn nested_references() {
        let mut object_a = Map::default();
        object_a.set("number", ValueContainer::from(42));
        object_a.set("obj", ValueContainer::from(Map::default()));

        // construct object_a as a value first
        let object_a_val = ValueContainer::new_value(object_a);

        // create object_b as a reference
        let object_b_ref = ValueContainer::new_reference(Map::default());

        // set object_a as property of b. This should create a reference to a clone of object_a that
        // is upgraded to a reference
        object_b_ref.with_maybe_reference(|b_ref| {
            b_ref.try_set_text_property("a", object_a_val.clone())
        });

        println!("Object B Reference: {:#?}", object_b_ref);

        // assert that the reference to object_a is set correctly
        object_b_ref
            .with_maybe_reference(|b_ref| {
                let object_a_ref =
                    b_ref.try_get_value_for_key("a").unwrap().unwrap();
                assert_structural_eq!(object_a_ref, object_a_val);
                // object_a_ref should be a reference
                assert_matches!(object_a_ref, ValueContainer::Reference(_));
                object_a_ref.with_maybe_reference(|a_ref| {
                    // object_a_ref.number should be a value
                    assert_matches!(
                        a_ref.try_get_value_for_key("number"),
                        Ok(Some(ValueContainer::Value(_)))
                    );
                    // object_a_ref.obj should be a reference
                    assert_matches!(
                        a_ref.try_get_value_for_key("obj"),
                        Ok(Some(ValueContainer::Reference(_)))
                    );
                });
            })
            .expect("object_b_ref should be a reference");
    }

    #[test]
    fn value_change_observe() {
        let int_ref = Reference::from(42);

        let observed_update: Rc<RefCell<Option<DIFUpdate>>> =
            Rc::new(RefCell::new(None));
        let observed_update_clone = Rc::clone(&observed_update);

        // Attach an observer to the reference
        int_ref
            .observe(move |update| {
                *observed_update_clone.borrow_mut() = Some(update.clone());
            })
            .expect("Failed to attach observer");

        // Update the value of the reference
        int_ref.try_set_value(43).expect("Failed to set value");

        // Verify the observed update matches the expected change
        let expected_update =
            DIFUpdate::Replace(DIFValue::from(&ValueContainer::from(43)));
        assert_eq!(*observed_update.borrow(), Some(expected_update));
    }
}
