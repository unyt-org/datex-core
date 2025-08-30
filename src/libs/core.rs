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
    Boolean = 2, // #core.boolean
    Integer = 3, // #core.integer
    Decimal = 4, // #core.decimal
    Text = 5, // #core.text
    Endpoint = 6, // #core.Endpoint
    Array = 7, // #core.Array
    Tuple = 8, // #core.Tuple
    Object = 9, // #core.Object
    Function = 10, // #core.Function
    // ...
}

impl From<CoreLibPointerId> for PointerAddress {
    fn from(id: CoreLibPointerId) -> Self {
        let id_bytes: [u8; 3] = (id as u64).to_le_bytes()[0..3].try_into().unwrap();
        PointerAddress::Internal(id_bytes)
    }
}

impl From<&PointerAddress> for CoreLibPointerId {
    fn from(address: &PointerAddress) -> Self {
        match address {
            PointerAddress::Internal(id_bytes) => {
                let mut id_array = [0u8; 8];
                id_array[0..3].copy_from_slice(id_bytes);
                let id = u64::from_le_bytes(id_array);
                match id {
                    0 => CoreLibPointerId::Core,
                    1 => CoreLibPointerId::Null,
                    2 => CoreLibPointerId::Boolean,
                    3 => CoreLibPointerId::Integer,
                    4 => CoreLibPointerId::Decimal,
                    5 => CoreLibPointerId::Text,
                    6 => CoreLibPointerId::Endpoint,
                    7 => CoreLibPointerId::Array,
                    8 => CoreLibPointerId::Tuple,
                    9 => CoreLibPointerId::Object,
                    10 => CoreLibPointerId::Function,
                    _ => panic!("Invalid CoreLibPointerId"),
                }
            }
            _ => panic!("CoreLibPointerId can only be created from Internal PointerAddress"),
        }
    }
}

/// Creates a new instance of the core library as a ValueContainer
/// and registers it in the provided memory instance using fixed internal pointer IDs.
pub fn load_core_lib(memory: &mut Memory) {
    let null = create_null_core_type(Some(memory));
    let boolean = create_boolean_core_type(Some(memory));
    let integer = create_integer_core_type(Some(memory));
    let decimal = create_decimal_core_type(Some(memory));
    let text = create_text_core_type(Some(memory));
    let endpoint = create_endpoint_core_type(Some(memory));
    let array = create_array_core_type(Some(memory));
    let tuple = create_tuple_core_type(Some(memory));
    let object = create_object_core_type(Some(memory));
    let function = create_function_core_type(Some(memory));

    // create #core object with properties
    let value = ValueContainer::from(Object::from_iter(vec![
        ("null".to_string(), null),
        ("boolean".to_string(), boolean),
        ("integer".to_string(), integer),
        ("decimal".to_string(), decimal),
        ("text".to_string(), text),
        ("endpoint".to_string(), endpoint),
        ("Array".to_string(), array),
        ("Tuple".to_string(), tuple),
        ("Object".to_string(), object),
        ("Function".to_string(), function),

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

/// Creates a new 'integer' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_integer_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new(
        "integer",
        &["i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "big"]
    ), CoreLibPointerId::Integer, memory)
}

/// Creates a new 'text' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_text_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new(
        "text",
        &["plain", "markdown", "html"]
    ), CoreLibPointerId::Text, memory)
}


/// Creates a new 'decimal' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_decimal_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new(
        "decimal",
        &["f32", "f64", "big"]
    ), CoreLibPointerId::Decimal, memory)
}

/// Creates a new 'boolean' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_boolean_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new("boolean", &[]), CoreLibPointerId::Boolean, memory)
}

/// Creates a new 'null' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_null_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new("null", &[]), CoreLibPointerId::Null, memory)
}

/// Creates a new 'endpoint' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_endpoint_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new("endpoint", &[]), CoreLibPointerId::Endpoint, memory)
}

/// Creates a new 'Object' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_object_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new("Object", &[]), CoreLibPointerId::Object, memory)
}

/// Creates a new 'Array' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_array_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new("Array", &[]), CoreLibPointerId::Array, memory)
}

/// Creates a new 'Tuple' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_tuple_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new("Tuple", &[]), CoreLibPointerId::Tuple, memory)
}

/// Creates a new 'Function' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_function_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type(TypeTag::new("Function", &[]), CoreLibPointerId::Function, memory)
}


/// Creates a new reference for a core type, similar to this DATEX script snippet:
/// ref $0 = Tag('integer', ('i8', 'i16', 'i32', 'i64', 'i128', 'u8', 'u16', 'u32', 'u64', 'u128'));
/// The reference is registered in the provided memory instance with a fixed internal pointer ID.
fn create_core_type(tag: TypeTag, id: CoreLibPointerId, memory: Option<&mut Memory>) -> ValueContainer {
    let value = ValueContainer::from(tag);
    // TODO: better solution for allowed_type here:
    let allowed_type = value.to_value().borrow().r#type().clone();
    let reference = Reference::new_from_value_container(
        value,
        allowed_type,
        Some(PointerAddress::from(id)),
        ReferenceMutability::Immutable
    );
    if let Some(memory) = memory {
        // register reference in memory
        memory.register_reference(reference.clone());
    }
    ValueContainer::Reference(reference)
}