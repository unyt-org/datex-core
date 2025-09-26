use crate::dif::DIFUpdate;
use crate::dif::value::DIFValueContainer;
use crate::runtime::Runtime;
use crate::values::pointer::PointerAddress;


pub trait DIFInterface {
    /// Applies a DIF update to the value at the given pointer address.
    fn update(&mut self, address: PointerAddress, update: DIFUpdate);
    /// Executes an apply operation, applying the `value` to the `callee`.
    fn apply(&mut self, callee: DIFValueContainer, value: DIFValueContainer);
    /// Creates a new pointer with the given DIF value and returns its address.
    fn create_pointer(&self, value: DIFValueContainer) -> PointerAddress;
    /// Starts observing changes to the pointer at the given address.
    /// As long as the pointer is observed, it will not be garbage collected.
    fn observe_pointer(&self, address: PointerAddress);
    /// Stops observing changes to the pointer at the given address.
    /// If no other references to the pointer exist, it may be garbage collected after this call.
    fn unobserve_pointer(&self, address: PointerAddress);
}

impl DIFInterface for Runtime {
    fn update(&mut self, address: PointerAddress, update: DIFUpdate) {
        todo!()
    }

    fn apply(&mut self, callee: DIFValueContainer, value: DIFValueContainer) {
        todo!()
    }

    fn create_pointer(&self, value: DIFValueContainer) -> PointerAddress {
        todo!()
    }

    fn observe_pointer(&self, address: PointerAddress) {
        todo!()
    }

    fn unobserve_pointer(&self, address: PointerAddress) {
        todo!()
    }
}