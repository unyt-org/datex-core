use core::prelude::rust_2024::*;
use core::result::Result;
use crate::references::reference::Reference;
use crate::references::type_reference::{
    NominalTypeDeclaration, TypeReference,
};
use crate::runtime::memory::Memory;
use crate::types::definition::TypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::r#type::Type;
use datex_core::values::core_values::map::Map;
use datex_core::values::pointer::PointerAddress;
use datex_core::values::value_container::ValueContainer;
use datex_macros::LibTypeString;
use core::cell::RefCell;
use crate::stdlib::collections::HashMap;
use core::iter::once;
use crate::stdlib::rc::Rc;
use strum::IntoEnumIterator;
use crate::stdlib::vec;
use crate::stdlib::vec::Vec;
use crate::stdlib::format;
use crate::stdlib::string::String;

thread_local! {
    pub static CORE_LIB_TYPES: HashMap<CoreLibPointerId, TypeContainer> = create_core_lib();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, LibTypeString)]
pub enum CoreLibPointerId {
    Core,                                // #core
    Type,                                // #core.type
    Null,                                // #core.null
    Boolean,                             // #core.boolean
    Integer(Option<IntegerTypeVariant>), // #core.integer
    Decimal(Option<DecimalTypeVariant>), // #core.decimal
    Text,                                // #core.text
    Endpoint,                            // #core.endpoint
    List,                                // #core.List
    Map,                                 // #core.Map
    Function,                            // #core.Function
    Unit,                                // #core.Unit
    Never,                               // #core.never
    Unknown,                             // #core.unknown
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
            CoreLibPointerId::Function => 5,
            CoreLibPointerId::Endpoint => 7,
            CoreLibPointerId::Text => 8,
            CoreLibPointerId::List => 9,
            CoreLibPointerId::Unit => 11,
            CoreLibPointerId::Map => 12,
            CoreLibPointerId::Never => 13,
            CoreLibPointerId::Unknown => 14,
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
        }
    }

    pub fn from_u16(id: u16) -> Option<Self> {
        match id {
            0 => Some(CoreLibPointerId::Core),
            1 => Some(CoreLibPointerId::Null),
            2 => Some(CoreLibPointerId::Type),
            3 => Some(CoreLibPointerId::Boolean),
            5 => Some(CoreLibPointerId::Function),
            7 => Some(CoreLibPointerId::Endpoint),
            8 => Some(CoreLibPointerId::Text),
            9 => Some(CoreLibPointerId::List),
            11 => Some(CoreLibPointerId::Unit),
            12 => Some(CoreLibPointerId::Map),
            13 => Some(CoreLibPointerId::Never),
            14 => Some(CoreLibPointerId::Unknown),

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
        let id_bytes: [u8; 3] =
            (id.to_u16() as u32).to_le_bytes()[0..3].try_into().unwrap();
        PointerAddress::Internal(id_bytes)
    }
}

impl TryFrom<&PointerAddress> for CoreLibPointerId {
    type Error = String;
    fn try_from(address: &PointerAddress) -> Result<Self, Self::Error> {
        match address {
            PointerAddress::Internal(id_bytes) => {
                let mut id_array = [0u8; 4];
                id_array[0..3].copy_from_slice(id_bytes);
                let id = u32::from_le_bytes(id_array);
                match CoreLibPointerId::from_u16(id as u16) {
                    Some(core_id) => Ok(core_id),
                    None => Err("Invalid CoreLibPointerId".to_string()),
                }
            }
            e => Err(format!(
                "CoreLibPointerId can only be created from Internal PointerAddress, got: {:?}",
                e
            )),
        }
    }
}

pub fn get_core_lib_type(id: impl Into<CoreLibPointerId>) -> TypeContainer {
    let id = id.into();
    if !has_core_lib_type(id.clone()) {
        core::panic!("Core lib type not found: {:?}", id);
    }
    CORE_LIB_TYPES.with(|core| core.get(&id).unwrap().clone())
}

pub fn get_core_lib_type_reference(
    id: impl Into<CoreLibPointerId>,
) -> Rc<RefCell<TypeReference>> {
    let type_container = get_core_lib_type(id);
    match type_container {
        TypeContainer::TypeReference(tr) => tr,
        _ => core::panic!("Core lib type is not a TypeReference"),
    }
}

fn has_core_lib_type<T>(id: T) -> bool
where
    T: Into<CoreLibPointerId>,
{
    CORE_LIB_TYPES.with(|core| core.contains_key(&id.into()))
}

/// Loads the core library into the provided memory instance.
pub fn load_core_lib(memory: &mut Memory) {
    CORE_LIB_TYPES.with(|core| {
        let structure = core
            .values()
            .map(|def| match def {
                TypeContainer::TypeReference(def) => {
                    let name = def
                        .borrow()
                        .nominal_type_declaration
                        .as_ref()
                        .unwrap()
                        .to_string();
                    let reference = Reference::TypeReference(def.clone());
                    memory.register_reference(&reference);
                    (name, ValueContainer::Reference(reference))
                }
                _ => core::panic!("Core lib type is not a TypeReference"),
            })
            .collect::<Vec<(String, ValueContainer)>>();

        // TODO #455: dont store variants as separate entries in core_struct (e.g., integer/u8, integer/i32, only keep integer)
        // Import variants directly by variant access operator from base type (e.g., integer -> integer/u8)
        let core_struct =
            Reference::from(ValueContainer::from(Map::from_iter(structure)));
        core_struct.set_pointer_address(CoreLibPointerId::Core.into());
        memory.register_reference(&core_struct);
    });
}

/// Creates a new instance of the core library as a ValueContainer
/// including all core types as properties.
pub fn create_core_lib() -> HashMap<CoreLibPointerId, TypeContainer> {
    let integer = integer();
    let decimal = decimal();
    vec![
        r#type(),
        text(),
        list(),
        boolean(),
        endpoint(),
        unit(),
        never(),
        unknown(),
        map(),
        null(),
    ]
    .into_iter()
    .chain(once(integer.clone()))
    .chain(
        IntegerTypeVariant::iter()
            .map(|variant| integer_variant(integer.1.clone(), variant)),
    )
    .chain(once(decimal.clone()))
    .chain(
        DecimalTypeVariant::iter()
            .map(|variant| decimal_variant(decimal.1.clone(), variant)),
    )
    .collect::<HashMap<CoreLibPointerId, TypeContainer>>()
}

type CoreLibTypeDefinition = (CoreLibPointerId, TypeContainer);
pub fn r#type() -> CoreLibTypeDefinition {
    create_core_type("type", None, None, CoreLibPointerId::Type)
}
pub fn null() -> CoreLibTypeDefinition {
    create_core_type("null", None, None, CoreLibPointerId::Null)
}
pub fn list() -> CoreLibTypeDefinition {
    create_core_type("List", None, None, CoreLibPointerId::List)
}
pub fn map() -> CoreLibTypeDefinition {
    create_core_type("Map", None, None, CoreLibPointerId::Map)
}

pub fn unit() -> CoreLibTypeDefinition {
    create_core_type("Unit", None, None, CoreLibPointerId::Unit)
}

pub fn never() -> CoreLibTypeDefinition {
    create_core_type("never", None, None, CoreLibPointerId::Never)
}

pub fn unknown() -> CoreLibTypeDefinition {
    create_core_type("unknown", None, None, CoreLibPointerId::Unknown)
}

pub fn boolean() -> CoreLibTypeDefinition {
    create_core_type("boolean", None, None, CoreLibPointerId::Boolean)
}

pub fn decimal() -> CoreLibTypeDefinition {
    create_core_type("decimal", None, None, CoreLibPointerId::Decimal(None))
}

pub fn decimal_variant(
    base_type: TypeContainer,
    variant: DecimalTypeVariant,
) -> CoreLibTypeDefinition {
    let variant_name = variant.as_ref().to_string();
    create_core_type(
        "decimal",
        Some(variant_name),
        Some(base_type),
        CoreLibPointerId::Decimal(Some(variant)),
    )
}
pub fn endpoint() -> CoreLibTypeDefinition {
    create_core_type("endpoint", None, None, CoreLibPointerId::Endpoint)
}

pub fn text() -> CoreLibTypeDefinition {
    create_core_type("text", None, None, CoreLibPointerId::Text)
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
            core::panic!("Base type must be a TypeReference")
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
    use crate::values::core_values::endpoint::Endpoint;

    use super::*;
    use itertools::Itertools;
    use crate::stdlib::{assert_matches::assert_matches, str::FromStr};

    #[test]
    fn core_lib() {
        assert!(has_core_lib_type(CoreLibPointerId::Endpoint));
        assert!(has_core_lib_type(CoreLibPointerId::Null));
        assert!(has_core_lib_type(CoreLibPointerId::Boolean));
        assert!(has_core_lib_type(CoreLibPointerId::Integer(None)));
        assert!(has_core_lib_type(CoreLibPointerId::Decimal(None)));
        for variant in IntegerTypeVariant::iter() {
            assert!(has_core_lib_type(CoreLibPointerId::Integer(Some(
                variant
            ))));
        }
        for variant in DecimalTypeVariant::iter() {
            assert!(has_core_lib_type(CoreLibPointerId::Decimal(Some(
                variant
            ))));
        }
    }

    #[test]
    fn debug() {
        let mut memory = Memory::new(Endpoint::LOCAL);
        load_core_lib(&mut memory);
        println!(
            "{}",
            memory
                .get_value_reference(&CoreLibPointerId::Core.into())
                .unwrap()
                .borrow()
                .value_container
        );
    }

    #[test]
    fn core_lib_type_addresses() {
        let integer_base = "integer";
        let integer_u8 = "integer/u8";
        let integer_i32 = "integer/i32";
        let decimal_base = "decimal";
        let decimal_f64 = "decimal/f64";

        assert_eq!(
            CoreLibPointerId::from_str(integer_base),
            Ok(CoreLibPointerId::Integer(None))
        );
        assert_eq!(
            CoreLibPointerId::from_str(integer_u8),
            Ok(CoreLibPointerId::Integer(Some(IntegerTypeVariant::U8)))
        );
        assert_eq!(
            CoreLibPointerId::from_str(integer_i32),
            Ok(CoreLibPointerId::Integer(Some(IntegerTypeVariant::I32)))
        );
        assert_eq!(
            CoreLibPointerId::from_str(decimal_base),
            Ok(CoreLibPointerId::Decimal(None))
        );
        assert_eq!(
            CoreLibPointerId::from_str(decimal_f64),
            Ok(CoreLibPointerId::Decimal(Some(DecimalTypeVariant::F64)))
        );

        assert_eq!(CoreLibPointerId::Integer(None).to_string(), integer_base);
        assert_eq!(
            CoreLibPointerId::Integer(Some(IntegerTypeVariant::U8)).to_string(),
            integer_u8
        );
        assert_eq!(
            CoreLibPointerId::Integer(Some(IntegerTypeVariant::I32))
                .to_string(),
            integer_i32
        );
        assert_eq!(CoreLibPointerId::Decimal(None).to_string(), decimal_base);
        assert_eq!(
            CoreLibPointerId::Decimal(Some(DecimalTypeVariant::F64))
                .to_string(),
            decimal_f64
        );
    }

    #[test]
    fn core_lib_pointer_id_conversion() {
        let core_id = CoreLibPointerId::Core;
        let pointer_address: PointerAddress = core_id.clone().into();
        let converted_id: CoreLibPointerId =
            (&pointer_address).try_into().unwrap();
        assert_eq!(core_id, converted_id);

        let boolean_id = CoreLibPointerId::Boolean;
        let pointer_address: PointerAddress = boolean_id.clone().into();
        let converted_id: CoreLibPointerId =
            (&pointer_address).try_into().unwrap();
        assert_eq!(boolean_id, converted_id);

        let integer_id =
            CoreLibPointerId::Integer(Some(IntegerTypeVariant::I32));
        let pointer_address: PointerAddress = integer_id.clone().into();
        let converted_id: CoreLibPointerId =
            (&pointer_address).try_into().unwrap();
        assert_eq!(integer_id, converted_id);

        let decimal_id =
            CoreLibPointerId::Decimal(Some(DecimalTypeVariant::F64));
        let pointer_address: PointerAddress = decimal_id.clone().into();
        let converted_id: CoreLibPointerId =
            (&pointer_address).try_into().unwrap();
        assert_eq!(decimal_id, converted_id);

        let type_id = CoreLibPointerId::Type;
        let pointer_address: PointerAddress = type_id.clone().into();
        let converted_id: CoreLibPointerId =
            (&pointer_address).try_into().unwrap();
        assert_eq!(type_id, converted_id);
    }

    #[test]
    fn base_type_simple() {
        // integer -> integer -> integer ...
        let integer_type = get_core_lib_type(CoreLibPointerId::Integer(None));
        let integer_base = integer_type.base_type();
        assert_matches!(integer_base, TypeContainer::TypeReference(_));
        assert_eq!(integer_base.to_string(), "integer");

        let base = integer_base.base_type();
        assert_matches!(base, TypeContainer::TypeReference(_));
        assert_eq!(base.to_string(), "integer");

        assert_eq!(integer_base, base);
    }

    #[test]
    fn base_type_complex() {
        // integer/u8 -> integer -> integer -> integer ...
        let integer_u8_type = get_core_lib_type(CoreLibPointerId::Integer(
            Some(IntegerTypeVariant::U8),
        ));
        assert_matches!(integer_u8_type, TypeContainer::TypeReference(_));
        assert_eq!(integer_u8_type.to_string(), "integer/u8");

        let integer = integer_u8_type.base_type();
        assert_matches!(integer, TypeContainer::TypeReference(_));
        assert_eq!(integer.to_string(), "integer");
        assert_ne!(integer, integer_u8_type);

        let integer_again = integer.base_type();
        assert_matches!(integer_again, TypeContainer::TypeReference(_));
        assert_eq!(integer_again.to_string(), "integer");
        assert_eq!(integer_again, integer);
    }

    #[ignore]
    #[test]
    fn print_core_lib_addresses_as_hex() {
        let sorted_entries = CORE_LIB_TYPES.with(|core| {
            core.keys()
                .map(|k| (k.clone(), PointerAddress::from(k.clone())))
                .sorted_by_key(|(_, address)| address.bytes().to_vec())
                .collect::<Vec<_>>()
        });
        for (core_lib_id, address) in sorted_entries {
            println!("{:?}: {}", core_lib_id, address);
        }
    }
}
