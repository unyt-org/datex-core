use datex_core::values::core_values::object::Object;
use datex_core::values::pointer::PointerAddress;
use datex_core::values::reference::{Reference, ReferenceMutability};
use datex_core::values::value_container::ValueContainer;
use crate::runtime::memory::Memory;
use crate::values::core_value::TypeTag;

/// Fixed mapping of internal pointer IDs for core library values.
pub enum CoreLibPointerId {
    Core = 0, // #core
    Null = 1, // #core.null
    Integer = 2, // #core.integer
    Boolean = 3, // #core.boolean
    // ...
}

impl From<CoreLibPointerId> for PointerAddress {
    fn from(id: CoreLibPointerId) -> Self {
        let id_bytes: [u8; 3] = (id as u64).to_le_bytes()[0..3].try_into().unwrap();
        PointerAddress::Internal(id_bytes)
    }
}

/// Creates a new instance of the core library as a ValueContainer
/// and registers it in the provided memory instance using fixed internal pointer IDs.
pub fn load_core_lib(memory: &mut Memory) {
    let null = create_core_type(TypeTag::new("null", &[]), CoreLibPointerId::Null, memory);
    let boolean = create_core_type(TypeTag::new("boolean", &[]), CoreLibPointerId::Boolean, memory);

    let integer = create_core_type(TypeTag::new(
        "integer",
        &["i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128"]
    ), CoreLibPointerId::Integer, memory);

    // create #core object with properties
    let value = ValueContainer::from(Object::from_iter(vec![
        ("null".to_string(), null),
        ("integer".to_string(), integer),
        ("boolean".to_string(), boolean),
        // TODO: add other core types here...
    ]));
    // TODO: better solution for allowed_type here:
    let allowed_type = value.to_value().borrow().r#type().clone();
    let reference = Reference::new_from_value_container(
        value,
        allowed_type,
        Some(PointerAddress::from(CoreLibPointerId::Core)),
        ReferenceMutability::Immutable
    );
    // register reference to #core in memory
    memory.register_reference(reference);
}

/// Creates a new reference for a core type, similar to this DATEX script snippet:
/// ref $0 = Tag('integer', ('i8', 'i16', 'i32', 'i64', 'i128', 'u8', 'u16', 'u32', 'u64', 'u128'));
/// The reference is registered in the provided memory instance with a fixed internal pointer ID.
fn create_core_type(tag: TypeTag, id: CoreLibPointerId, memory: &mut Memory) -> ValueContainer {
    let value = ValueContainer::from(tag);
    // TODO: better solution for allowed_type here:
    let allowed_type = value.to_value().borrow().r#type().clone();
    let reference = Reference::new_from_value_container(
        value,
        allowed_type,
        Some(PointerAddress::from(id)),
        ReferenceMutability::Immutable
    );
    // register reference in memory
    memory.register_reference(reference.clone());
    ValueContainer::Reference(reference)
}