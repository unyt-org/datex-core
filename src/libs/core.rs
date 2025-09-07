use crate::runtime::memory::Memory;
use crate::values::core_value::TypeTag;
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::r#type::r#type::Type;
use datex_core::values::core_values::object::Object;
use datex_core::values::pointer::PointerAddress;
use datex_core::values::reference::{Reference, ReferenceMutability};
use datex_core::values::value_container::ValueContainer;

/// Fixed mapping of internal pointer IDs for core library values.
pub enum CoreLibPointerId {
    Core,                                // #core
    Null,                                // #core.null
    Boolean,                             // #core.boolean
    Integer(Option<IntegerTypeVariant>), // #core.integer
    Decimal(Option<DecimalTypeVariant>), // #core.decimal
    Text,                                // #core.text
    Endpoint,                            // #core.Endpoint
    Array,                               // #core.Array
    Object,                              // #core.Object
    Function,                            // #core.Function
}

impl CoreLibPointerId {
    const INTEGER_BASE: u16 = 100;
    const DECIMAL_BASE: u16 = 300;

    pub fn to_u16(&self) -> u16 {
        match self {
            CoreLibPointerId::Core => 0,
            CoreLibPointerId::Null => 1,
            CoreLibPointerId::Boolean => 2,
            CoreLibPointerId::Integer(None) => INTEGER_BASE,
            CoreLibPointerId::Integer(Some(v)) => {
                let v: u8 = (*v).into();
                CoreLibPointerId::Integer(None).to_u16() + v as u16
            }
            CoreLibPointerId::Decimal(None) => DECIMAL_BASE,
            CoreLibPointerId::Decimal(Some(v)) => {
                let v: u8 = (*v).into();
                CoreLibPointerId::Decimal(None).to_u16() + v as u16
            }
            _ => panic!("Unsupported CoreLibPointerId variant for to_u64"),
        }
    }
    pub fn from_u16(id: u16) -> Option<Self> {
        match id {
            0 => Some(CoreLibPointerId::Core),
            1 => Some(CoreLibPointerId::Null),
            2 => Some(CoreLibPointerId::Boolean),

            Self::INTEGER_BASE => Some(CoreLibPointerId::Integer(None)),
            n if (Self::INTEGER_BASE + 1..Self::DECIMAL_BASE).contains(&n) => {
                IntegerVariant::try_from((n - Self::INTEGER_BASE) as u8)
                    .ok()
                    .map(|v| CoreLibPointerId::Integer(Some(v)))
            }

            Self::DECIMAL_BASE => Some(CoreLibPointerId::Decimal(None)),
            n if n > Self::DECIMAL_BASE => {
                DecimalVariant::try_from((n - Self::DECIMAL_BASE) as u8)
                    .ok()
                    .map(|v| CoreLibPointerId::Decimal(Some(v)))
            }

            _ => None,
        }
    }
}

impl From<CoreLibPointerId> for PointerAddress {
    fn from(id: CoreLibPointerId) -> Self {
        let id_bytes: [u8; 3] = id.to_u16().to_le_bytes()[0..3]
            .try_into()
            .expect("Failed to convert u16 to [u8; 3]");
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
                match CoreLibPointerId::from_u16(id as u16) {
                    Some(core_id) => core_id,
                    None => panic!("Invalid CoreLibPointerId"),
                }
            }
            _ => panic!(
                "CoreLibPointerId can only be created from Internal PointerAddress"
            ),
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
    let allowed_type = value.to_value().borrow().actual_type().clone();
    let reference = Reference::new_from_value_container(
        value,
        allowed_type,
        Some(PointerAddress::from(CoreLibPointerId::Core)),
        ReferenceMutability::Immutable,
    );
    // register reference to #core in memory
    memory.register_reference(reference);
}

/// Creates a new 'integer' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_integer_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new(
            "integer",
            &[
                "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64",
                "u128", "big",
            ],
        ),
        CoreLibPointerId::Integer,
        memory,
    )
}

/// Creates a new 'text' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_text_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("text", &["plain", "markdown", "html"]),
        CoreLibPointerId::Text,
        memory,
    )
}

/// Creates a new 'decimal' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_decimal_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("decimal", &["f32", "f64", "big"]),
        CoreLibPointerId::Decimal,
        memory,
    )
}

/// Creates a new 'boolean' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_boolean_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("boolean", &[]),
        CoreLibPointerId::Boolean,
        memory,
    )
}

/// Creates a new 'null' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_null_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("null", &[]),
        CoreLibPointerId::Null,
        memory,
    )
}

/// Creates a new 'endpoint' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_endpoint_core_type(
    memory: Option<&mut Memory>,
) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("endpoint", &[]),
        CoreLibPointerId::Endpoint,
        memory,
    )
}

/// Creates a new 'Object' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_object_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("Object", &[]),
        CoreLibPointerId::Object,
        memory,
    )
}

/// Creates a new 'Array' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_array_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("Array", &[]),
        CoreLibPointerId::Array,
        memory,
    )
}

/// Creates a new 'Tuple' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_tuple_core_type(memory: Option<&mut Memory>) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("Tuple", &[]),
        CoreLibPointerId::Tuple,
        memory,
    )
}

/// Creates a new 'Function' core type reference.
/// Note: this method should never be called directly, only use for testing purposes.
pub fn create_function_core_type(
    memory: Option<&mut Memory>,
) -> ValueContainer {
    create_core_type_old(
        TypeTag::new("Function", &[]),
        CoreLibPointerId::Function,
        memory,
    )
}

/// Creates a new reference for a core type, similar to this DATEX script snippet:
/// ref $0 = Tag('integer', ('i8', 'i16', 'i32', 'i64', 'i128', 'u8', 'u16', 'u32', 'u64', 'u128'));
/// The reference is registered in the provided memory instance with a fixed internal pointer ID.
fn create_core_type_old(
    tag: TypeTag,
    id: CoreLibPointerId,
    memory: Option<&mut Memory>,
) -> ValueContainer {
    todo!();
    // let value = ValueContainer::from(tag);
    // // TODO: better solution for allowed_type here:
    // let allowed_type = value.to_value().borrow().actual_type().clone();
    // let reference = Reference::new_from_value_container(
    //     value,
    //     allowed_type,
    //     Some(PointerAddress::from(id)),
    //     ReferenceMutability::Immutable,
    // );
    // if let Some(memory) = memory {
    //     // register reference in memory
    //     memory.register_reference(reference.clone());
    // }
    // ValueContainer::Reference(reference)
}

/// Creates a core type without a specific variant, e.g., 'integer' without variant.
fn create_core_type(name: &str, definition: Reference) -> Type {
    Type::nominal(name, definition, None)
}

/// Creates a core type with a specific variant, e.g., 'integer' with variant 'i32'.
fn create_core_type_with_variant(
    name: &str,
    definition: Reference,
    variant: &str,
) -> Type {
    Type::nominal(name, definition, Some(variant))
}

/// Converts a core type into a Reference with the given internal pointer ID.
fn type_as_reference(r#type: Type, id: CoreLibPointerId) -> Reference {
    let value = ValueContainer::from(r#type);
    Reference::new_from_value_container(
        value,
        r#type,
        Some(PointerAddress::from(id)),
        ReferenceMutability::Immutable,
    )
}

/// Registers a core type in memory and returns it as a ValueContainer.
fn register_core_type(
    r#type: Type,
    id: CoreLibPointerId,
    memory: &mut Memory,
) -> ValueContainer {
    let reference = type_as_reference(r#type, id);
    memory.register_reference(reference.clone());
    ValueContainer::Reference(reference)
}
