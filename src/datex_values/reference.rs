use std::cell::{Ref, RefCell};
use std::hash::{Hash, Hasher};
use std::ops::{Deref};
use std::rc::Rc;
use crate::datex_values::pointer::Pointer;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::ValueContainer;
use super::{datex_type::CoreValueType};

#[derive(Clone, Debug, Eq)]
pub struct Reference(pub Rc<RefCell<ReferenceData>>);

impl PartialEq for Reference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Hash for Reference {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Rc::as_ptr(&self.0); // gets *const RefCell<ReferenceData>
        ptr.hash(state); // hash the address
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
