use crate::runtime::memory::Memory;
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::r#type::Type;
use crate::values::core_values::r#type::definition::TypeDefinition;
use crate::values::reference::Reference;
use crate::values::type_container::TypeContainer;
use crate::values::type_reference::{NominalTypeDeclaration, TypeReference};
use datex_core::values::core_values::object::Object;
use datex_core::values::pointer::PointerAddress;
use datex_core::values::value_container::ValueContainer;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    pub static CORE_LIB_TYPES: HashMap<CoreLibPointerId, TypeContainer> = create_core_lib();
}

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
            CoreLibPointerId::Object => 4,
            CoreLibPointerId::Function => 5,
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
            _ => panic!("Unsupported CoreLibPointerId"),
        }
    }
    pub fn from_u16(id: u16) -> Option<Self> {
        match id {
            0 => Some(CoreLibPointerId::Core),
            1 => Some(CoreLibPointerId::Null),
            2 => Some(CoreLibPointerId::Type),
            3 => Some(CoreLibPointerId::Boolean),
            4 => Some(CoreLibPointerId::Object),
            5 => Some(CoreLibPointerId::Function),

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

pub fn get_core_lib_value(id: impl Into<CoreLibPointerId>) -> TypeContainer {
    CORE_LIB_TYPES
        .with(|core| core.get(&id.into()).expect("Core type not found").clone())
}

/// Loads the core library into the provided memory instance.
pub fn load_core_lib(memory: &mut Memory) {
    CORE_LIB_TYPES.with(|core| {
        let object = core
            .iter()
            .map(|(_id, def)| match def {
                TypeContainer::TypeReference(def) => {
                    let name = def
                        .borrow()
                        .nominal_type_declaration
                        .as_ref()
                        .unwrap()
                        .to_string();
                    let reference = Reference::TypeReference(def.clone());
                    memory.register_reference(reference.clone());
                    (name, ValueContainer::Reference(reference))
                }
                _ => panic!("Core lib type is not a TypeReference"),
            })
            .collect::<Vec<(String, ValueContainer)>>();
        let core_object = Reference::from(ValueContainer::from(
            Object::from_iter(object.into_iter()),
        ));
        core_object.set_pointer_address(CoreLibPointerId::Core.into());
        memory.register_reference(core_object);
    });
}

/// Creates a new instance of the core library as a ValueContainer
/// including all core types as properties.
pub fn create_core_lib() -> HashMap<CoreLibPointerId, TypeContainer> {
    let integer = integer();

    [
        null(),
        object(),
        // integers
        integer.clone(),
        integer_variant(integer.1.clone(), IntegerTypeVariant::U8),
        integer_variant(integer.1.clone(), IntegerTypeVariant::U16),
        integer_variant(integer.1.clone(), IntegerTypeVariant::U32),
        integer_variant(integer.1.clone(), IntegerTypeVariant::U64),
        integer_variant(integer.1.clone(), IntegerTypeVariant::I8),
        integer_variant(integer.1.clone(), IntegerTypeVariant::I16),
        integer_variant(integer.1.clone(), IntegerTypeVariant::I32),
        integer_variant(integer.1.clone(), IntegerTypeVariant::I64),
    ]
    .into_iter()
    .collect::<HashMap<CoreLibPointerId, TypeContainer>>()
}

type CoreLibTypeDefinition = (CoreLibPointerId, TypeContainer);

pub fn null() -> CoreLibTypeDefinition {
    create_core_type("null", None, None, CoreLibPointerId::Null)
}
pub fn object() -> CoreLibTypeDefinition {
    create_core_type("Object", None, None, CoreLibPointerId::Object)
}

pub fn integer() -> CoreLibTypeDefinition {
    create_core_type("integer", None, None, CoreLibPointerId::Integer(None))
}
pub fn integer_variant(
    base_type: TypeContainer,
    variant: IntegerTypeVariant,
) -> CoreLibTypeDefinition {
    let variant_name = variant.as_ref().to_string();
    create_core_type(
        "integer",
        Some(variant_name),
        Some(base_type),
        CoreLibPointerId::Integer(Some(variant)),
    )
}

/// Creates a core type with the given parameters.
fn create_core_type(
    name: &str,
    variant: Option<String>,
    base_type: Option<TypeContainer>,
    pointer_id: CoreLibPointerId,
) -> CoreLibTypeDefinition {
    let base_type_ref = match base_type {
        Some(TypeContainer::TypeReference(reference)) => Some(reference),
        Some(TypeContainer::Type(_)) => {
            panic!("Base type must be a TypeReference")
        }
        None => None,
    };
    (
        pointer_id.clone(),
        TypeContainer::TypeReference(Rc::new(RefCell::new(TypeReference {
            nominal_type_declaration: Some(NominalTypeDeclaration {
                name: name.to_string(),
                variant,
            }),
            type_value: Type {
                base_type: base_type_ref,
                reference_mutability: None,
                type_definition: TypeDefinition::Unit,
            },
            pointer_address: Some(PointerAddress::from(pointer_id)),
        }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_value::CoreValue;
    use std::assert_matches::assert_matches;

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

        let type_id = CoreLibPointerId::Type;
        let pointer_address: PointerAddress = type_id.clone().into();
        let converted_id: CoreLibPointerId = (&pointer_address).into();
        assert_eq!(type_id, converted_id);
    }

    #[test]
    fn base_type() {
        let integer_type = get_core_lib_value(CoreLibPointerId::Integer(None));
        let integer_base = integer_type.base_type();
        assert_eq!(integer_base.to_string(), "integer");
        let base = integer_base.base_type();
        assert_eq!(base.to_string(), "integer");

        let integer_u8_type =
            get_core_lib_value(CoreLibPointerId::Integer(Some(
                IntegerTypeVariant::U8,
            )));
        assert_eq!(integer_u8_type.to_string(), "integer/u8");
        let integer_u8_base = integer_u8_type.base_type();
        assert_eq!(integer_u8_base.to_string(), "integer");
        let base = integer_u8_base.base_type();
        assert_eq!(base.to_string(), "integer");
    }
}
