use crate::values::core_value::borrowed_core_value::BorrowedCoreValue;
use core::hash::{Hash, Hasher};

impl Hash for BorrowedCoreValue<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            BorrowedCoreValue::Uninitialized => state.write_u8(0),
            BorrowedCoreValue::Null => state.write_u8(1),
            BorrowedCoreValue::Boolean(b) => b.hash(state),
            BorrowedCoreValue::Integer(i) => i.hash(state),
            BorrowedCoreValue::TypedInteger(ti) => ti.hash(state),
            BorrowedCoreValue::Decimal(d) => d.hash(state),
            BorrowedCoreValue::TypedDecimal(td) => td.hash(state),
            BorrowedCoreValue::Text(t) => t.hash(state),
            BorrowedCoreValue::Endpoint(e) => e.hash(state),
            BorrowedCoreValue::List(l) => l.hash(state),
            BorrowedCoreValue::Map(m) => m.hash(state),
            BorrowedCoreValue::Type(t) => t.hash(state),
            BorrowedCoreValue::EntityTypeDefinition(etd) => etd.hash(state),
            BorrowedCoreValue::Callable(c) => c.hash(state),
            BorrowedCoreValue::Range(r) => r.hash(state),
            BorrowedCoreValue::Box(b) => b.hash(state),
            BorrowedCoreValue::Instant(i) => i.hash(state),
        }
    }
}
