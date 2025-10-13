use crate::references::observers::Observer;
use crate::references::reference::ReferenceMutability;
use crate::traits::value_eq::ValueEq;
use crate::types::type_container::TypeContainer;
use crate::utils::freemap::FreeHashMap;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

pub struct ValueReference {
    /// the value that this reference points to
    pub value_container: ValueContainer,
    /// pointer id, can be initialized as None for local pointers
    pub pointer_address: Option<PointerAddress>,
    /// custom type for the pointer that the Datex value is allowed to reference
    pub allowed_type: TypeContainer,
    /// list of observer callbacks
    pub observers: FreeHashMap<u32, Observer>,
    pub mutability: ReferenceMutability,
}

impl Default for ValueReference {
    fn default() -> Self {
        ValueReference {
            value_container: ValueContainer::Value(Value::null()),
            pointer_address: None,
            allowed_type: TypeContainer::null(),
            observers: FreeHashMap::new(),
            mutability: ReferenceMutability::Immutable,
        }
    }
}

impl ValueReference {
    pub fn new(
        value_container: ValueContainer,
        pointer_address: Option<PointerAddress>,
        allowed_type: TypeContainer,
        mutability: ReferenceMutability,
    ) -> Self {
        ValueReference {
            value_container,
            pointer_address,
            allowed_type,
            observers: FreeHashMap::new(),
            mutability,
        }
    }
}

impl Debug for ValueReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferenceData")
            .field("value_container", &self.value_container)
            .field("pointer", &self.pointer_address)
            .field("allowed_type", &self.allowed_type)
            .field("observers", &self.observers.len())
            .finish()
    }
}

impl PartialEq for ValueReference {
    fn eq(&self, other: &Self) -> bool {
        // Two value references are equal if their value containers are equal
        self.value_container.value_eq(&other.value_container)
    }
}

impl ValueReference {
    pub fn pointer_address(&self) -> &Option<PointerAddress> {
        &self.pointer_address
    }

    pub fn current_value_container(&self) -> &ValueContainer {
        &self.value_container
    }

    pub fn resolve_current_value(&self) -> Rc<RefCell<Value>> {
        self.value_container.to_value()
    }

    pub fn is_mutable(&self) -> bool {
        matches!(self.mutability, ReferenceMutability::Mutable)
    }

    pub fn is_final(&self) -> bool {
        matches!(self.mutability, ReferenceMutability::Final)
    }
}
