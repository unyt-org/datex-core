use crate::values::value_container::borrowed_value_container::BorrowedValueContainer;
use core::hash::{Hash, Hasher};

impl Hash for BorrowedValueContainer<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            BorrowedValueContainer::Local(value) => value.hash(state),
            BorrowedValueContainer::Shared(shared) => shared.hash(state),
        }
    }
}
