use crate::{
    traits::datex_hash::impl_datex_hash, values::core_value::CoreValue,
};
use core::hash::{Hash, Hasher};

impl Hash for CoreValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CoreValue::Uninitialized => state.write_u8(0),
            CoreValue::Null => state.write_u8(1),
            CoreValue::Boolean(b) => b.hash(state),
            CoreValue::Integer(i) => i.hash(state),
            CoreValue::TypedInteger(ti) => ti.hash(state),
            CoreValue::Decimal(d) => d.hash(state),
            CoreValue::TypedDecimal(td) => td.hash(state),
            CoreValue::Text(t) => t.hash(state),
            CoreValue::Endpoint(e) => e.hash(state),
            CoreValue::List(l) => l.hash(state),
            CoreValue::Map(m) => m.hash(state),
            CoreValue::Type(t) => t.hash(state),
            CoreValue::EntityTypeDefinition(etd) => etd.hash(state),
            CoreValue::Callable(c) => c.hash(state),
            CoreValue::Range(r) => r.hash(state),
            CoreValue::Box(b) => b.hash(state),
            CoreValue::Instant(i) => i.hash(state),
        }
    }
}

impl_datex_hash!(CoreValue);
