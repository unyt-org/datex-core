use crate::collections::HashMap;
use crate::references::reference::Reference;
use crate::references::type_reference::{
    NominalTypeDeclaration, TypeReference,
};
use crate::runtime::memory::Memory;
use crate::stdlib::boxed::Box;
use crate::stdlib::format;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec;
use crate::stdlib::vec::Vec;
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::callable::{
    CallableBody, CallableKind, CallableSignature,
};
use crate::values::core_values::decimal::typed_decimal::DecimalTypeVariant;
use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::values::core_values::map::Map;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;
use core::iter::once;
use core::prelude::rust_2024::*;
use core::result::Result;
use datex_macros::LibTypeString;
use log::info;
use strum::IntoEnumIterator;

type CoreLibTypes = HashMap<CoreLibPointerId, Type>;
type CoreLibVals = HashMap<CoreLibPointerId, ValueContainer>;

#[cfg_attr(not(feature = "embassy_runtime"), thread_local)]
pub static mut CORE_LIB_TYPES: Option<CoreLibTypes> = None;

#[cfg_attr(not(feature = "embassy_runtime"), thread_local)]
pub static mut CORE_LIB_VALS: Option<CoreLibVals> = None;

fn with_full_core_lib<R>(
    handler: impl FnOnce(&CoreLibTypes, &CoreLibVals) -> R,
) -> R {
    unsafe {
        if CORE_LIB_TYPES.is_none() {
            CORE_LIB_TYPES.replace(create_core_lib_types());
        }
        if CORE_LIB_VALS.is_none() {
            CORE_LIB_VALS.replace(create_core_lib_vals());
        }
        handler(
            CORE_LIB_TYPES.as_ref().unwrap_unchecked(),
            CORE_LIB_VALS.as_ref().unwrap_unchecked(),
        )
    }
}

fn with_core_lib_types<R>(handler: impl FnOnce(&CoreLibTypes) -> R) -> R {
    unsafe {
        if CORE_LIB_TYPES.is_none() {
            CORE_LIB_TYPES.replace(create_core_lib_types());
        }
        handler(CORE_LIB_TYPES.as_ref().unwrap_unchecked())
    }
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
    Callable,                            // #core.Callable
    Unit,                                // #core.Unit
    Never,                               // #core.never
    Unknown,                             // #core.unknown
    Print, // #core.print (function, might be removed later)
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
            CoreLibPointerId::Callable => 5,
            CoreLibPointerId::Endpoint => 7,
            CoreLibPointerId::Text => 8,
            CoreLibPointerId::List => 9,
            CoreLibPointerId::Unit => 11,
            CoreLibPointerId::Map => 12,
            CoreLibPointerId::Never => 13,
            CoreLibPointerId::Unknown => 14,
            CoreLibPointerId::Print => 15,
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
            5 => Some(CoreLibPointerId::Callable),
            7 => Some(CoreLibPointerId::Endpoint),
            8 => Some(CoreLibPointerId::Text),
            9 => Some(CoreLibPointerId::List),
            11 => Some(CoreLibPointerId::Unit),
            12 => Some(CoreLibPointerId::Map),
            13 => Some(CoreLibPointerId::Never),
            14 => Some(CoreLibPointerId::Unknown),
            15 => Some(CoreLibPointerId::Print),

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

pub fn get_core_lib_type(id: impl Into<CoreLibPointerId>) -> Type {
    with_core_lib_types(|core_lib_types| {
        core_lib_types.get(&id.into()).unwrap().clone()
    })
}

pub fn get_core_lib_type_reference(
    id: impl Into<CoreLibPointerId>,
) -> Rc<RefCell<TypeReference>> {
    let type_container = get_core_lib_type(id);
    match type_container.type_definition {
        TypeDefinition::Reference(tr) => tr,
        _ => core::panic!("Core lib type is not a TypeReference"),
    }
}

/// Retrieves either a core library type or value by its CoreLibPointerId.
pub fn get_core_lib_value(
    id: impl Into<CoreLibPointerId>,
) -> Option<ValueContainer> {
    let id = id.into();
    with_full_core_lib(|core_lib_types, core_lib_values| {
        // try types first
        if let Some(ty) = core_lib_types.get(&id) {
            match &ty.type_definition {
                TypeDefinition::Reference(tr) => {
                    Some(ValueContainer::Reference(Reference::TypeReference(
                        tr.clone(),
                    )))
                }
                _ => core::panic!("Core lib type is not a TypeReference"),
            }
        } else if let Some(val) = core_lib_values.get(&id) {
            Some(val.clone())
        } else {
            None
        }
    })
}

pub fn get_core_lib_type_definition(
    id: impl Into<CoreLibPointerId>,
) -> TypeDefinition {
    get_core_lib_type(id).type_definition
}

fn has_core_lib_type<T>(id: T) -> bool
where
    T: Into<CoreLibPointerId>,
{
    with_core_lib_types(|core_lib_types| {
        core_lib_types.contains_key(&id.into())
    })
}

/// Loads the core library into the provided memory instance.
pub fn load_core_lib(memory: &mut Memory) {
    with_full_core_lib(|core_lib_types, core_lib_values| {
        let mut types_structure = core_lib_types
            .values()
            .map(|ty| match &ty.type_definition {
                TypeDefinition::Reference(type_reference) => {
                    let name = type_reference
                        .borrow()
                        .nominal_type_declaration
                        .as_ref()
                        .unwrap()
                        .to_string();
                    let reference =
                        Reference::TypeReference(type_reference.clone());
                    memory.register_reference(&reference);
                    (name, ValueContainer::Reference(reference))
                }
                _ => core::panic!("Core lib type is not a TypeReference"),
            })
            .collect::<Vec<(String, ValueContainer)>>();

        // add core lib values
        for (name, val) in core_lib_values.iter() {
            let name = name.to_string();
            types_structure.push((name, val.clone()));
        }

        // TODO #455: dont store variants as separate entries in core_struct (e.g., integer/u8, integer/i32, only keep integer)
        // Import variants directly by variant access operator from base type (e.g., integer -> integer/u8)
        let core_struct = Reference::from(ValueContainer::from(
            Map::from_iter(types_structure),
        ));
        core_struct.set_pointer_address(CoreLibPointerId::Core.into());
        memory.register_reference(&core_struct);
    });
}

/// Creates a new instance of the core library as a ValueContainer
/// including all core types as properties.
pub fn create_core_lib_types() -> HashMap<CoreLibPointerId, Type> {
    let integer = integer();
    let decimal = decimal();
    vec![
        ty(),
        text(),
        list(),
        boolean(),
        endpoint(),
        unit(),
        never(),
        unknown(),
        map(),
        null(),
        callable(),
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
    .collect::<HashMap<CoreLibPointerId, Type>>()
}

pub fn create_core_lib_vals() -> HashMap<CoreLibPointerId, ValueContainer> {
    vec![print()]
        .into_iter()
        .collect::<HashMap<CoreLibPointerId, ValueContainer>>()
}

type CoreLibTypeDefinition = (CoreLibPointerId, Type);
pub fn ty() -> CoreLibTypeDefinition {
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

pub fn callable() -> CoreLibTypeDefinition {
    create_core_type("Callable", None, None, CoreLibPointerId::Callable)
}

pub fn decimal_variant(
    base_type: Type,
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
    base_type: Type,
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

pub fn print() -> (CoreLibPointerId, ValueContainer) {
    (
        CoreLibPointerId::Print,
        ValueContainer::Value(Value::callable(
            Some("print".to_string()),
            CallableSignature {
                kind: CallableKind::Function,
                parameter_types: vec![],
                rest_parameter_type: Some((
                    Some("values".to_string()),
                    Box::new(Type::unknown()),
                )),
                return_type: None,
                yeet_type: None,
            },
            CallableBody::Native(|mut args: &[ValueContainer]| {
                // TODO #680: add I/O abstraction layer / interface

                let mut output = String::new();

                // if first argument is a string value, print it directly
                if let Some(ValueContainer::Value(Value {
                    inner: CoreValue::Text(text),
                    ..
                })) = args.get(0)
                {
                    output.push_str(&text.0);
                    // remove first argument from args
                    args = &args[1..];
                    // if there are still arguments, add a space
                    if !args.is_empty() {
                        output.push(' ');
                    }
                }

                #[cfg(feature = "decompiler")]
                let args_string = args
                    .iter()
                    .map(|v| {
                        crate::decompiler::decompile_value(
                            v,
                            crate::decompiler::DecompileOptions::colorized(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                #[cfg(not(feature = "decompiler"))]
                let args_string = args
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                output.push_str(&args_string);

                #[cfg(feature = "std")]
                println!("[PRINT] {}", output);
                info!("[PRINT] {}", output);
                Ok(None)
            }),
        )),
    )
}

/// Creates a core type with the given parameters.
fn create_core_type(
    name: &str,
    variant: Option<String>,
    base_type: Option<Type>,
    pointer_id: CoreLibPointerId,
) -> CoreLibTypeDefinition {
    let base_type_ref = match base_type {
        Some(Type {
            type_definition: TypeDefinition::Reference(reference),
            ..
        }) => Some(reference),
        Some(_) => {
            core::panic!("Base type must be a Reference")
        }
        None => None,
    };
    (
        pointer_id.clone(),
        Type::new(
            TypeDefinition::reference(Rc::new(RefCell::new(TypeReference {
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
            None,
        ),
    )
}

#[cfg(test)]
mod tests {
    use crate::values::core_values::endpoint::Endpoint;

    use super::*;
    use crate::stdlib::{assert_matches::assert_matches, str::FromStr};
    use itertools::Itertools;

    #[test]
    fn core_lib() {
        assert!(has_core_lib_type(CoreLibPointerId::Endpoint));
        assert!(has_core_lib_type(CoreLibPointerId::Null));
        assert!(has_core_lib_type(CoreLibPointerId::Boolean));
        assert!(has_core_lib_type(CoreLibPointerId::Integer(None)));
        assert!(has_core_lib_type(CoreLibPointerId::Decimal(None)));
        assert!(has_core_lib_type(CoreLibPointerId::Type));
        assert!(has_core_lib_type(CoreLibPointerId::Text));
        assert!(has_core_lib_type(CoreLibPointerId::List));
        assert!(has_core_lib_type(CoreLibPointerId::Map));
        assert!(has_core_lib_type(CoreLibPointerId::Callable));
        assert!(has_core_lib_type(CoreLibPointerId::Unit));
        assert!(has_core_lib_type(CoreLibPointerId::Never));
        assert!(has_core_lib_type(CoreLibPointerId::Unknown));
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
        let integer_base = integer_type.base_type_reference();
        assert_eq!(integer_base.unwrap().borrow().to_string(), "integer");
    }

    #[test]
    fn base_type_complex() {
        // integer/u8 -> integer -> integer -> integer ...
        let integer_u8_type = get_core_lib_type(CoreLibPointerId::Integer(
            Some(IntegerTypeVariant::U8),
        ));
        assert_eq!(integer_u8_type.to_string(), "integer/u8");

        let integer = integer_u8_type.base_type_reference();
        assert_eq!(integer.unwrap().borrow().to_string(), "integer");
    }

    #[ignore]
    #[test]
    fn print_core_lib_addresses_as_hex() {
        with_full_core_lib(|core_lib_types, _| {
            let sorted_entries = core_lib_types
                .keys()
                .map(|k| (k.clone(), PointerAddress::from(k.clone())))
                .sorted_by_key(|(_, address)| address.bytes().to_vec())
                .collect::<Vec<_>>();
            for (core_lib_id, address) in sorted_entries {
                println!("{:?}: {}", core_lib_id, address);
            }
        });
    }

    #[test]
    #[ignore]
    /// Generates a TypeScript mapping of core type addresses to their names.
    /// Run this test and copy the output into `src/dif/definitions.ts`.
    ///
    /// `cargo test create_core_type_ts_mapping -- --show-output --ignored`
    fn create_core_type_ts_mapping() {
        let core_lib = create_core_lib_types();
        let mut core_lib: Vec<(CoreLibPointerId, PointerAddress)> = core_lib
            .keys()
            .map(|key| (key.clone(), PointerAddress::from(key.clone())))
            .collect();
        core_lib.sort_by_key(|(key, _)| {
            PointerAddress::from(key.clone()).bytes().to_vec()
        });

        println!("export const CoreTypeAddress = {{");
        for (core_lib_id, address) in core_lib {
            println!(
                "    {}: \"{}\",",
                core_lib_id.to_string().replace("/", "_"),
                address.to_string().strip_prefix("$").unwrap()
            );
        }
        println!("}} as const;");
    }
}
