use crate::runtime::memory::Memory;
use crate::values::core_value::TypeTag;
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::r#type::r#type::{Type, TypeDefinition};
use crate::values::value::Value;
use datex_core::values::core_values::object::Object;
use datex_core::values::pointer::PointerAddress;
use datex_core::values::reference::{Reference, ReferenceMutability};
use datex_core::values::value_container::ValueContainer;
use webrtc::mdns::message::name;

/// Fixed mapping of internal pointer IDs for core library values.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreLibPointerId {
    Core,                                // #core
    Type,                                // #core.type
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
            CoreLibPointerId::Type => 2,
            CoreLibPointerId::Boolean => 3,
            CoreLibPointerId::Integer(None) => Self::INTEGER_BASE,
            CoreLibPointerId::Integer(Some(v)) => {
                let v: u8 = (*v).into();
                CoreLibPointerId::Integer(None).to_u16() + v as u16
            }
            CoreLibPointerId::Decimal(None) => Self::DECIMAL_BASE,
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
            2 => Some(CoreLibPointerId::Type),
            3 => Some(CoreLibPointerId::Boolean),

            Self::INTEGER_BASE => Some(CoreLibPointerId::Integer(None)),
            n if (Self::INTEGER_BASE + 1..Self::DECIMAL_BASE).contains(&n) => {
                IntegerTypeVariant::try_from((n - Self::INTEGER_BASE) as u8)
                    .ok()
                    .map(|v| CoreLibPointerId::Integer(Some(v)))
            }

            Self::DECIMAL_BASE => Some(CoreLibPointerId::Decimal(None)),
            n if n > Self::DECIMAL_BASE => {
                DecimalTypeVariant::try_from((n - Self::DECIMAL_BASE) as u8)
                    .ok()
                    .map(|v| CoreLibPointerId::Decimal(Some(v)))
            }

            _ => None,
        }
    }
}

impl From<CoreLibPointerId> for PointerAddress {
    fn from(id: CoreLibPointerId) -> Self {
        let id_bytes: [u8; 3] = (id.to_u16() as u32).to_le_bytes()[0..3]
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
// pub fn load_core_lib(memory: &mut Memory) {
//     let null = create_null_core_type(Some(memory));
//     let boolean = create_boolean_core_type(Some(memory));
//     let integer = create_integer_core_type(Some(memory));
//     let decimal = create_decimal_core_type(Some(memory));
//     let text = create_text_core_type(Some(memory));
//     let endpoint = create_endpoint_core_type(Some(memory));
//     let array = create_array_core_type(Some(memory));
//     let tuple = create_tuple_core_type(Some(memory));
//     let object = create_object_core_type(Some(memory));
//     let function = create_function_core_type(Some(memory));

//     // create #core object with properties
//     let value = ValueContainer::from(Object::from_iter(vec![
//         ("null".to_string(), null),
//         ("boolean".to_string(), boolean),
//         ("integer".to_string(), integer),
//         ("decimal".to_string(), decimal),
//         ("text".to_string(), text),
//         ("endpoint".to_string(), endpoint),
//         ("Array".to_string(), array),
//         ("Tuple".to_string(), tuple),
//         ("Object".to_string(), object),
//         ("Function".to_string(), function),
//         // TODO: add other core types here...
//     ]));
//     // TODO: better solution for allowed_type here:
//     let allowed_type = value.to_value().borrow().actual_type().clone();
//     let reference = Reference::new_from_value_container(
//         value,
//         allowed_type,
//         Some(PointerAddress::from(CoreLibPointerId::Core)),
//         ReferenceMutability::Immutable,
//     );
//     // register reference to #core in memory
//     memory.register_reference(reference);
// }

/// Loads the core library into the provided memory instance.
pub fn load_core_lib(memory: &mut Memory) {
    let core = create_core_lib();
    let reference = Reference::new_from_value_container(
        core.clone(),
        core.actual_type(),
        Some(PointerAddress::from(CoreLibPointerId::Core)),
        ReferenceMutability::Immutable,
    );
    memory.register_reference(reference);
}

pub fn base_type() -> Reference {
    type_as_reference(
        Type::nominal(
            "Type",
            Reference::new_from_value_container(
                ValueContainer::from(Object::default()),
                Type::structural(Object::default()),
                None,
                ReferenceMutability::Immutable,
            ),
            None,
        ),
        CoreLibPointerId::Type,
    )
}

/// Creates a new instance of the core library as a ValueContainer
/// including all core types as properties.
pub fn create_core_lib() -> ValueContainer {
    let mut core = Object::default();
    let types = vec![null()];
    for r#type in types {
        if let Type {
            type_definition: TypeDefinition::Nominal(e),
            ..
        } = &r#type.borrow().value_container.actual_type()
        {
            let type_name = e.name.clone();
            core.set(
                type_name.as_str(),
                ValueContainer::Reference(r#type.clone()),
            );
        }
    }

    ValueContainer::from(core)
}

pub fn null() -> Reference {
    println!("Creating core type: null");
    type_as_reference(
        create_core_type("null", base_type()),
        CoreLibPointerId::Null,
    )
}
pub fn nullType() -> Type {
    null().borrow().value_container.actual_type().clone()
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
    let value = ValueContainer::from(r#type.clone());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let null = null();
        print!("{:?}", null);
    }

    #[test]
    fn create_core_lib_success() {
        let core = create_core_lib();
        print!("{:#?}", core);
    }

    #[test]
    fn test_core_lib_pointer_id_conversion() {
        let core_id = CoreLibPointerId::Core;
        let pointer_address: PointerAddress = core_id.clone().into();
        let converted_id: CoreLibPointerId = (&pointer_address).into();
        assert_eq!(core_id, converted_id);

        let boolean_id = CoreLibPointerId::Boolean;
        let pointer_address: PointerAddress = boolean_id.clone().into();
        let converted_id: CoreLibPointerId = (&pointer_address).into();
        assert_eq!(boolean_id, converted_id);

        let integer_id =
            CoreLibPointerId::Integer(Some(IntegerTypeVariant::I32));
        let pointer_address: PointerAddress = integer_id.clone().into();
        let converted_id: CoreLibPointerId = (&pointer_address).into();
        assert_eq!(integer_id, converted_id);

        let decimal_id =
            CoreLibPointerId::Decimal(Some(DecimalTypeVariant::F64));
        let pointer_address: PointerAddress = decimal_id.clone().into();
        let converted_id: CoreLibPointerId = (&pointer_address).into();
        assert_eq!(decimal_id, converted_id);
    }
}
