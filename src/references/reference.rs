use crate::references::type_reference::{
    NominalTypeDeclaration, TypeReference,
};
use crate::types::type_container::TypeContainer;
use crate::values::core_value::CoreValue;

use crate::references::value_reference::ValueReference;
use crate::values::core_values::map::{Map, MapSetError};
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::traits::identity::Identity;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug)]
pub enum AccessError {
    ImmutableReference,
    InvalidOperation(String),
    PropertyNotFound(String),
    CanNotUseReferenceAsKey,
    IndexOutOfBounds(u32),
    InvalidPropertyKeyType(String),
    MapSetError(MapSetError),
}

impl From<MapSetError> for AccessError {
    fn from(err: MapSetError) -> Self {
        AccessError::MapSetError(err)
    }
}

impl Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessError::MapSetError(err) => {
                write!(f, "Map set error: {}", err)
            }
            AccessError::ImmutableReference => {
                write!(f, "Cannot modify an immutable reference")
            }
            AccessError::InvalidOperation(op) => {
                write!(f, "Invalid operation: {}", op)
            }
            AccessError::PropertyNotFound(prop) => {
                write!(f, "Property not found: {}", prop)
            }
            AccessError::CanNotUseReferenceAsKey => {
                write!(f, "Cannot use a reference as a property key")
            }
            AccessError::IndexOutOfBounds(index) => {
                write!(f, "Index out of bounds: {}", index)
            }
            AccessError::InvalidPropertyKeyType(ty) => {
                write!(f, "Invalid property key type: {}", ty)
            }
        }
    }
}

#[derive(Debug)]
pub enum TypeError {
    TypeMismatch {
        expected: TypeContainer,
        found: TypeContainer,
    },
}
impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::TypeMismatch { expected, found } => write!(
                f,
                "Type mismatch: expected {}, found {}",
                expected, found
            ),
        }
    }
}

#[derive(Debug)]
pub enum AssignmentError {
    ImmutableReference,
    TypeError(TypeError),
}

impl Display for AssignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignmentError::ImmutableReference => {
                write!(f, "Cannot assign to an immutable reference")
            }
            AssignmentError::TypeError(e) => write!(f, "Type error: {}", e),
        }
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TryFromPrimitive,
)]
#[repr(u8)]
pub enum ReferenceMutability {
    Mutable = 0,
    Immutable = 1,
    Final = 2,
}

impl Display for ReferenceMutability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceMutability::Mutable => write!(f, "&mut"),
            ReferenceMutability::Final => write!(f, "&final"),
            ReferenceMutability::Immutable => write!(f, "&"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Reference {
    ValueReference(Rc<RefCell<ValueReference>>),
    TypeReference(Rc<RefCell<TypeReference>>),
}

impl Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reference::ValueReference(vr) => {
                let vr = vr.borrow();
                write!(f, "{} {}", vr.mutability, vr.value_container)
            }
            Reference::TypeReference(tr) => {
                let tr = tr.borrow();
                write!(f, "{}", tr)
            }
        }
    }
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

impl Display for ReferenceFromValueContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceFromValueContainerError::InvalidType => {
                write!(
                    f,
                    "Cannot create reference from value container: invalid type"
                )
            }
            ReferenceFromValueContainerError::MutableTypeReference => {
                write!(f, "Cannot create mutable reference to type")
            }
        }
    }
}

impl Reference {
    /// Runs a closure with the current value of this reference.
    pub(crate) fn with_value<R, F: FnOnce(&mut Value) -> R>(
        &self,
        f: F,
    ) -> Option<R> {
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

    pub(crate) fn with_value_unchecked<R, F: FnOnce(&mut Value) -> R>(
        &self,
        f: F,
    ) -> R {
        self.with_value(f).unwrap()
    }

    /// Checks if the reference has property access.
    /// This is true for objects and structs, arrays and lists and text.
    /// For other types, this returns false.
    /// Note that this does not check if a specific property exists, only if property access is
    /// generally possible.
    pub fn supports_property_access(&self) -> bool {
        self.with_value(|value| {
            matches!(
                value.inner,
                CoreValue::Map(_) | CoreValue::List(_) | CoreValue::Text(_)
            )
        })
        .unwrap_or(false)
    }

    /// Checks if the reference has text property access.
    /// This is true for structs.
    pub fn supports_text_property_access(&self) -> bool {
        self.with_value(|value| matches!(value.inner, CoreValue::Map(_)))
            .unwrap_or(false)
    }

    /// Checks if the reference has numeric property access.
    /// This is true for arrays and lists and text.
    pub fn supports_numeric_property_access(&self) -> bool {
        self.with_value(|value| {
            matches!(
                value.inner,
                CoreValue::Map(_) | CoreValue::List(_) | CoreValue::Text(_)
            )
        })
        .unwrap_or(false)
    }

    /// Checks if the reference supports push operation
    pub fn supports_push(&self) -> bool {
        self.with_value(|value| matches!(value.inner, CoreValue::List(_)))
            .unwrap_or(false)
    }
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
    pub fn is_mutable(&self) -> bool {
        self.mutability() == ReferenceMutability::Mutable
    }

    /// Creates a new reference from a value container
    pub fn try_new_from_value_container(
        value_container: ValueContainer,
        allowed_type: Option<TypeContainer>,
        maybe_pointer_id: Option<PointerAddress>,
        mutability: ReferenceMutability,
    ) -> Result<Self, ReferenceFromValueContainerError> {
        /**
        * val a: &mut text = &mut "hello" // Reference<mutable, type: text>
        * val b: &&mut text = &a     // Reference<immutable, type: Reference<&mut text>>
        #*/
        // FIXME implement type check
        Ok(match value_container {
            ValueContainer::Reference(ref reference) => {
                let allowed_type =
                    allowed_type.unwrap_or_else(|| reference.allowed_type());
                // TODO: make sure allowed type is superset of reference's allowed type
                Reference::ValueReference(Rc::new(RefCell::new(
                    ValueReference::new(
                        value_container,
                        maybe_pointer_id,
                        allowed_type,
                        mutability,
                    ),
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
                        if mutability == ReferenceMutability::Mutable {
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
                            ValueReference::new(
                                ValueContainer::Value(value),
                                maybe_pointer_id,
                                allowed_type,
                                mutability,
                            ),
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

    pub fn try_final_from(
        value_container: ValueContainer,
    ) -> Result<Self, ReferenceFromValueContainerError> {
        // TODO: don't allow creating final reference from &mut/& references
        Reference::try_new_from_value_container(
            value_container,
            None,
            None,
            ReferenceMutability::Final,
        )
    }

    /// Collapses the reference chain to most inner reference to which this reference points.
    pub fn collapse_reference_chain(&self) -> Reference {
        match self {
            Reference::TypeReference(tr) => Reference::TypeReference(Rc::new(
                RefCell::new(tr.borrow().collapse_reference_chain()),
            )),
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

    // TODO: no clone?
    pub fn value_container(&self) -> ValueContainer {
        match self {
            Reference::ValueReference(vr) => {
                vr.borrow().value_container.clone()
            }
            Reference::TypeReference(tr) => ValueContainer::Value(Value::from(
                CoreValue::Type(tr.borrow().type_value.clone()),
            )),
        }
    }

    /// upgrades all inner combined values (e.g. object properties) to references
    pub fn upgrade_inner_combined_values_to_references(&self) {
        self.with_value(|value| {
            match &mut value.inner {
                CoreValue::Map(map) => {
                    // Iterate over all properties and upgrade them to references
                    for (_, prop) in map.into_iter() {
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
    pub fn bind_child(&self, child: ValueContainer) -> ValueContainer {
        // Ensure the child is a reference if it is a combined value

        child.upgrade_combined_value_to_reference()
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

    /// Returns a non-final reference to the ValueReference if this is a non-final ValueReference.
    pub fn non_final_reference(&self) -> Option<Rc<RefCell<ValueReference>>> {
        match self {
            Reference::TypeReference(_) => None,
            Reference::ValueReference(vr) => {
                if !vr.borrow().is_final() {
                    Some(vr.clone())
                } else {
                    None
                }
            }
        }
    }
}

/// Getter for references
impl Reference {
    /// Gets a property on the value if applicable (e.g. for map and structs)
    pub fn try_get_property<T: Into<ValueContainer>>(
        &self,
        key: T,
    ) -> Result<ValueContainer, AccessError> {
        let key = key.into();
        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut map) => {
                    // If the value is an object, get the property
                    Ok(map
                        .get(&key)
                        .ok_or(AccessError::PropertyNotFound(key.to_string()))?
                        .clone())
                }
                _ => {
                    // If the value is not an object, we cannot get a property
                    Err(AccessError::InvalidOperation(
                        "Cannot get property".to_string(),
                    ))
                }
            }
        })
        .unwrap_or(Err(AccessError::InvalidOperation(
            "Cannot get property on invalid reference".to_string(),
        )))
    }

    /// Gets a text property from the value if applicable (e.g. for structs)
    pub fn try_get_text_property(
        &self,
        key: &str,
    ) -> Result<ValueContainer, AccessError> {
        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut struct_val) => struct_val
                    .get_text(&key)
                    .ok_or_else(|| {
                        AccessError::PropertyNotFound(key.to_string())
                    })
                    .cloned(),
                _ => {
                    // If the value is not an object, we cannot get a property
                    Err(AccessError::InvalidOperation(
                        "Cannot get property".to_string(),
                    ))
                }
            }
        })
        .unwrap_or(Err(AccessError::InvalidOperation(
            "Cannot get property on invalid reference".to_string(),
        )))
    }

    /// Gets a numeric property from the value if applicable (e.g. for arrays, lists and text)
    pub fn get_numeric_property(
        &self,
        index: u32,
    ) -> Result<ValueContainer, AccessError> {
        self.with_value(|value| match value.inner {
            CoreValue::List(ref mut list) => list
                .get(index)
                .cloned()
                .ok_or(AccessError::IndexOutOfBounds(index)),
            CoreValue::Text(ref text) => {
                let char = text
                    .char_at(index as usize)
                    .ok_or(AccessError::IndexOutOfBounds(index))?;
                Ok(ValueContainer::from(char.to_string()))
            }
            _ => Err(AccessError::InvalidOperation(
                "Cannot get numeric property".to_string(),
            )),
        })
        .unwrap_or(Err(AccessError::InvalidOperation(
            "Cannot get numeric property on invalid reference".to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::memory::Memory;
    use crate::values::traits::value_eq::ValueEq;
    use crate::{assert_identical, assert_structural_eq, assert_value_eq};
    use datex_core::values::core_values::map::Map;
    use std::assert_matches::assert_matches;

    #[test]
    fn property() {
        let mut object = Map::default();
        object.set("name", ValueContainer::from("Jonas"));
        object.set("age", ValueContainer::from(30));
        let reference = Reference::from(ValueContainer::from(object));
        assert_eq!(
            reference.try_get_property("name").unwrap(),
            ValueContainer::from("Jonas")
        );
        assert_eq!(
            reference.try_get_property("age").unwrap(),
            ValueContainer::from(30)
        );
        assert!(reference.try_get_property("nonexistent").is_err());
        assert_matches!(
            reference.try_get_property("nonexistent"),
            Err(AccessError::PropertyNotFound(_))
        );
    }

    #[test]
    fn text_property() {
        let struct_val = Map::from(vec![
            ("name".to_string(), ValueContainer::from("Jonas")),
            ("age".to_string(), ValueContainer::from(30)),
        ]);
        let reference = Reference::from(ValueContainer::from(struct_val));
        assert_eq!(
            reference.try_get_text_property("name").unwrap(),
            ValueContainer::from("Jonas")
        );
        assert_eq!(
            reference.try_get_text_property("age").unwrap(),
            ValueContainer::from(30)
        );
        assert!(reference.try_get_text_property("nonexistent").is_err());
        assert_matches!(
            reference.try_get_text_property("nonexistent"),
            Err(AccessError::PropertyNotFound(_))
        );
    }

    #[test]
    fn numeric_property() {
        let array = vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ];
        let reference = Reference::from(ValueContainer::from(array));

        assert_eq!(
            reference.get_numeric_property(0).unwrap(),
            ValueContainer::from(1)
        );
        assert_eq!(
            reference.get_numeric_property(1).unwrap(),
            ValueContainer::from(2)
        );
        assert_eq!(
            reference.get_numeric_property(2).unwrap(),
            ValueContainer::from(3)
        );
        assert!(reference.get_numeric_property(3).is_err());

        assert_matches!(
            reference.get_numeric_property(100),
            Err(AccessError::IndexOutOfBounds(100))
        );

        let text_ref = Reference::from(ValueContainer::from("hello"));
        assert_eq!(
            text_ref.get_numeric_property(1).unwrap(),
            ValueContainer::from("e".to_string())
        );
        assert!(text_ref.get_numeric_property(5).is_err());
        assert_matches!(
            text_ref.get_numeric_property(100),
            Err(AccessError::IndexOutOfBounds(100))
        );
    }

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
        let memory = &RefCell::new(Memory::default());

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
            b_ref
                .try_set_property("a".into(), object_a_val.clone(), memory)
                .unwrap();
        });

        println!("Object B Reference: {:#?}", object_b_ref);

        // assert that the reference to object_a is set correctly
        object_b_ref
            .with_maybe_reference(|b_ref| {
                let object_a_ref = b_ref.try_get_property("a").unwrap();
                assert_structural_eq!(object_a_ref, object_a_val);
                // object_a_ref should be a reference
                assert_matches!(object_a_ref, ValueContainer::Reference(_));
                object_a_ref.with_maybe_reference(|a_ref| {
                    // object_a_ref.number should be a value
                    assert_matches!(
                        a_ref.try_get_property("number"),
                        Ok(ValueContainer::Value(_))
                    );
                    // object_a_ref.obj should be a reference
                    assert_matches!(
                        a_ref.try_get_property("obj"),
                        Ok(ValueContainer::Reference(_))
                    );
                });
            })
            .expect("object_b_ref should be a reference");
    }
}
