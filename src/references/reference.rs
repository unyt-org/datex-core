use crate::references::type_reference::{
    NominalTypeDeclaration, TypeReference,
};
use crate::types::type_container::TypeContainer;
use crate::values::core_value::CoreValue;

use crate::references::value_reference::ValueReference;
use crate::runtime::execution::ExecutionError;
use crate::traits::apply::Apply;
use crate::traits::identity::Identity;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::values::core_values::map::{Map, MapAccessError};
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use core::cell::RefCell;
use core::fmt::Display;
use crate::stdlib::hash::{Hash, Hasher};
use crate::stdlib::rc::Rc;

#[derive(Debug)]
pub enum AccessError {
    ImmutableReference,
    InvalidOperation(String),
    PropertyNotFound(String),
    CanNotUseReferenceAsKey,
    IndexOutOfBounds(u32),
    InvalidPropertyKeyType(String),
    MapAccessError(Box<MapAccessError>),
}

impl From<MapAccessError> for AccessError {
    fn from(err: MapAccessError) -> Self {
        AccessError::MapAccessError(Box::new(err))
    }
}

impl Display for AccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AccessError::MapAccessError(err) => {
                write!(f, "Map access error: {}", err)
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
    TypeError(Box<TypeError>),
}

impl Display for AssignmentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
}

pub mod mutability_as_int {
    use super::ReferenceMutability;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        value: &ReferenceMutability,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            (ReferenceMutability::Mutable) => serializer.serialize_u8(0),
            (ReferenceMutability::Immutable) => serializer.serialize_u8(1),
        }
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<ReferenceMutability, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = u8::deserialize(deserializer)?;
        Ok(match opt {
            (0) => (ReferenceMutability::Mutable),
            (1) => (ReferenceMutability::Immutable),
            (x) => {
                return Err(D::Error::custom(format!(
                    "invalid mutability code: {}",
                    x
                )));
            }
        })
    }
}
pub mod mutability_option_as_int {
    use super::ReferenceMutability;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        value: &Option<ReferenceMutability>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(ReferenceMutability::Mutable) => serializer.serialize_u8(0),
            Some(ReferenceMutability::Immutable) => serializer.serialize_u8(1),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<ReferenceMutability>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<u8>::deserialize(deserializer)?;
        Ok(match opt {
            Some(0) => Some(ReferenceMutability::Mutable),
            Some(1) => Some(ReferenceMutability::Immutable),
            Some(x) => {
                return Err(D::Error::custom(format!(
                    "invalid mutability code: {}",
                    x
                )));
            }
            None => None,
        })
    }
}

impl Display for ReferenceMutability {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReferenceMutability::Mutable => write!(f, "&mut"),
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
                // FIXME #281: Implement value_eq for type and use here instead (recursive)
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
pub enum ReferenceCreationError {
    InvalidType,
    MutableTypeReference,
}

impl Display for ReferenceCreationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReferenceCreationError::InvalidType => {
                write!(
                    f,
                    "Cannot create reference from value container: invalid type"
                )
            }
            ReferenceCreationError::MutableTypeReference => {
                write!(f, "Cannot create mutable reference for type")
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

    // TODO #282: Mark as unsafe function
    pub(crate) fn with_value_unchecked<R, F: FnOnce(&mut Value) -> R>(
        &self,
        f: F,
    ) -> R {
        unsafe { self.with_value(f).unwrap_unchecked() }
    }

    /// Checks if the reference supports clear operation
    pub fn supports_clear(&self) -> bool {
        self.with_value(|value| match value.inner {
            CoreValue::Map(ref mut map) => match map {
                Map::Dynamic(_) => true,
                Map::Fixed(_) | Map::Structural(_) => false,
            },
            _ => false,
        })
        .unwrap_or(false)
    }

    /// Checks if the reference has property access.
    /// This is true for maps, lists and text.
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
    /// This is true for maps.
    pub fn supports_text_property_access(&self) -> bool {
        self.with_value(|value| matches!(value.inner, CoreValue::Map(_)))
            .unwrap_or(false)
    }

    /// Checks if the reference has numeric property access.
    /// This is true for maps, lists and text.
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

    /// Sets the pointer address of the reference.
    /// Panics if the reference already has a pointer address.
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

    /// Gets the mutability of the reference.
    /// TypeReferences are always immutable.
    pub(crate) fn mutability(&self) -> ReferenceMutability {
        match self {
            Reference::ValueReference(vr) => vr.borrow().mutability.clone(),
            Reference::TypeReference(_) => ReferenceMutability::Immutable,
        }
    }

    /// Checks if the reference is mutable.
    /// A reference is mutable if it is a mutable ValueReference and all references in the chain are mutable.
    /// TypeReferences are always immutable.
    /// FIXME #284: Do we really need this? Probably we already collapse the ref and then change it's value and perform
    /// the mutability check on the most inner ref.
    pub fn is_mutable(&self) -> bool {
        match self {
            Reference::TypeReference(_) => false, // type references are always immutable
            Reference::ValueReference(vr) => {
                let vr_borrow = vr.borrow();
                // if the current reference is immutable, whole chain is immutable
                if vr_borrow.mutability != ReferenceMutability::Mutable {
                    return false;
                }

                // otherwise, check if ref is pointing to another reference
                match &vr_borrow.value_container {
                    ValueContainer::Reference(inner) => inner.is_mutable(),
                    ValueContainer::Value(_) => true,
                }
            }
        }
    }

    /// Creates a new reference from a value container
    pub fn try_new_from_value_container(
        value_container: ValueContainer,
        allowed_type: Option<TypeContainer>,
        maybe_pointer_id: Option<PointerAddress>,
        mutability: ReferenceMutability,
    ) -> Result<Self, ReferenceCreationError> {
        // FIXME #285 implement type check
        Ok(match value_container {
            ValueContainer::Reference(ref reference) => {
                match reference {
                    Reference::ValueReference(vr) => {
                        let allowed_type = allowed_type.unwrap_or_else(|| {
                            vr.borrow().allowed_type.clone()
                        });
                        // TODO #286: make sure allowed type is superset of reference's allowed type
                        Reference::ValueReference(Rc::new(RefCell::new(
                            ValueReference::new(
                                value_container,
                                maybe_pointer_id,
                                allowed_type,
                                mutability,
                            ),
                        )))
                    }
                    Reference::TypeReference(tr) => {
                        if mutability == ReferenceMutability::Mutable {
                            return Err(
                                ReferenceCreationError::MutableTypeReference,
                            );
                        }
                        Reference::TypeReference(
                            TypeReference::anonymous(
                                Type::reference(tr.clone(), Some(mutability)),
                                maybe_pointer_id,
                            )
                            .as_ref_cell(),
                        )
                    }
                }
            }
            ValueContainer::Value(value) => {
                match value.inner {
                    // create TypeReference if the value is a Type
                    CoreValue::Type(type_value) => {
                        // TODO #287: allowed_type "Type" is also allowed
                        if allowed_type.is_some() {
                            return Err(ReferenceCreationError::InvalidType);
                        }
                        if mutability == ReferenceMutability::Mutable {
                            return Err(
                                ReferenceCreationError::MutableTypeReference,
                            );
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
    ) -> Result<Self, ReferenceCreationError> {
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
            // FIXME #288: Can we optimize this to avoid creating rc ref cells?
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
            // TODO #289: can we optimize this to avoid cloning the type value?
            Reference::TypeReference(tr) => Rc::new(RefCell::new(Value::from(
                CoreValue::Type(tr.borrow().type_value.clone()),
            ))),
        }
    }

    // TODO #290: no clone?
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

    /// upgrades all inner combined values (e.g. map properties) to references
    pub fn upgrade_inner_combined_values_to_references(&self) {
        self.with_value(|value| {
            match &mut value.inner {
                CoreValue::Map(map) => {
                    // Iterate over all properties and upgrade them to references
                    for (_, prop) in map.into_iter() {
                        // TODO #291: no clone here, implement some sort of map
                        *prop = self.bind_child(prop.clone());
                    }
                }
                // TODO #292: other combined value types should be added here
                _ => {
                    // If the value is not an map, we do not need to upgrade anything
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
            Reference::TypeReference(_) => todo!("#293 type Type"),
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
            Reference::TypeReference(tr) => todo!("#294 type Type"),
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

    /// Returns a mutable reference to the ValueReference if this is a mutable ValueReference.
    pub fn mutable_reference(&self) -> Option<Rc<RefCell<ValueReference>>> {
        match self {
            Reference::TypeReference(_) => None,
            Reference::ValueReference(vr) => {
                if vr.borrow().is_mutable() {
                    Some(vr.clone())
                } else {
                    None
                }
            }
        }
    }

    /// Sets the value container of the reference if it is mutable.
    /// If the reference is immutable, an error is returned.
    pub fn set_value_container(
        &self,
        new_value_container: ValueContainer,
    ) -> Result<(), AssignmentError> {
        match &self {
            Reference::TypeReference(_) => {
                Err(AssignmentError::ImmutableReference)
            }
            Reference::ValueReference(vr) => {
                if self.is_mutable() {
                    // TODO #295: check type compatibility, handle observers
                    vr.borrow_mut().value_container = new_value_container;
                    Ok(())
                } else {
                    Err(AssignmentError::ImmutableReference)
                }
            }
        }
    }
}
/// Getter for references
impl Reference {
    /// Gets a property on the value if applicable (e.g. for map and structs)
    // FIXME #296 make this return a reference to a value container
    // Just for later as myRef.x += 1
    // key_ref = myRef.x // myRef.try_get_property("x".into())
    // key_val = &key_ref.value()
    // &key_ref.set_value(key_val + 1)
    // -> we could avoid some clones if so (as get, addition, set would all be a clone)
    pub fn try_get_property<T: Into<ValueContainer>>(
        &self,
        key: T,
    ) -> Result<ValueContainer, AccessError> {
        let key = key.into();
        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut map) => {
                    // If the value is an map, get the property
                    Ok(map
                        .get(&key)
                        .ok_or(AccessError::PropertyNotFound(key.to_string()))?
                        .clone())
                }
                _ => {
                    // If the value is not an map, we cannot get a property
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
                    .get_text(key)
                    .ok_or_else(|| {
                        AccessError::PropertyNotFound(key.to_string())
                    })
                    .cloned(),
                _ => {
                    // If the value is not an map, we cannot get a property
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

    /// Gets a numeric property from the value if applicable (e.g. for lists and text)
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

impl Apply for Reference {
    fn apply(
        &self,
        args: &[ValueContainer],
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        todo!("#297 Undescribed by author.")
    }

    fn apply_single(
        &self,
        arg: &ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        match self {
            Reference::TypeReference(tr) => tr.borrow().apply_single(arg),
            Reference::ValueReference(vr) => todo!("#298 Undescribed by author."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::global_context::{GlobalContext, set_global_context};
    use crate::runtime::memory::Memory;
    use crate::traits::value_eq::ValueEq;
    use crate::{assert_identical, assert_structural_eq, assert_value_eq};
    use datex_core::values::core_values::map::Map;
    use crate::stdlib::assert_matches::assert_matches;

    #[test]
    fn try_mut_from() {
        // creating a mutable reference from a value should work
        let value = ValueContainer::from(42);
        let reference = Reference::try_mut_from(value).unwrap();
        assert_eq!(reference.mutability(), ReferenceMutability::Mutable);

        // creating a mutable reference from a type should fail
        let type_value = ValueContainer::Reference(Reference::TypeReference(
            TypeReference::anonymous(Type::UNIT, None).as_ref_cell(),
        ));
        assert_matches!(
            Reference::try_mut_from(type_value),
            Err(ReferenceCreationError::MutableTypeReference)
        );
    }

    #[test]
    fn property() {
        let mut map = Map::default();
        map.set("name", ValueContainer::from("Jonas"));
        map.set("age", ValueContainer::from(30));
        let reference = Reference::from(ValueContainer::from(map));
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
        let list = vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ];
        let reference = Reference::from(ValueContainer::from(list));

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
        set_global_context(GlobalContext::native());
        let memory = &RefCell::new(Memory::default());

        let mut map_a = Map::default();
        map_a.set("number", ValueContainer::from(42));
        map_a.set("obj", ValueContainer::new_reference(Map::default()));

        // construct map_a as a value first
        let map_a_val = ValueContainer::new_value(map_a);

        // create map_b as a reference
        let map_b_ref = Reference::try_new_from_value_container(
            Map::default().into(),
            None,
            None,
            ReferenceMutability::Mutable,
        )
        .unwrap();

        // set map_a as property of b. This should create a reference to a clone of map_a that
        // is upgraded to a reference
        map_b_ref
            .try_set_property(0, "a".into(), map_a_val.clone(), memory)
            .unwrap();

        // assert that the reference to map_a is set correctly
        let map_a_ref = map_b_ref.try_get_property("a").unwrap();
        assert_structural_eq!(map_a_ref, map_a_val);
        // map_a_ref should be a reference
        assert_matches!(map_a_ref, ValueContainer::Reference(_));
        map_a_ref.with_maybe_reference(|a_ref| {
            // map_a_ref.number should be a value
            assert_matches!(
                a_ref.try_get_property("number"),
                Ok(ValueContainer::Value(_))
            );
            // map_a_ref.obj should be a reference
            assert_matches!(
                a_ref.try_get_property("obj"),
                Ok(ValueContainer::Reference(_))
            );
        });
    }
}
