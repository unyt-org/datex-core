use crate::values::value::borrowed_value::BorrowedValue;
use core::hash::{Hash, Hasher};

impl Hash for BorrowedValue<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            BorrowedValue::Core(core_self) => {
                core_self.hash(state);
            }
            BorrowedValue::Native(native_self) => {
                native_self.datex_hash(state);
            }
        }
    }
}
