use crate::values::{
    core_values::r#type::{
        descriptor::TypeDescriptor, path::TypePath, r#type::Type,
    },
    datex_type::CoreValueType,
};

fn create_core_type_internal(
    name: &str,
    core_type: CoreValueType,
    base: Option<Type>,
) -> Type {
    let variant: Option<String> = name.split('.').nth(1).map(|s| s.to_string());
    let name = name.split('.').next().unwrap_or(name);
    match base {
        Some(base_type) => Type::new_with_base(
            TypePath::new("core", name, variant),
            TypeDescriptor::Core(core_type),
            base_type.name,
        ),
        None => Type::new(
            TypePath::new("core", name, variant),
            TypeDescriptor::Core(core_type),
        ),
    }
}
fn create_core_type(name: &str, core_type: CoreValueType) -> Type {
    create_core_type_internal(name, core_type, None)
}
fn create_core_type_with_base(
    name: &str,
    core_type: CoreValueType,
    base: Type,
) -> Type {
    create_core_type_internal(name, core_type, Some(base))
}

pub fn integer() -> Type {
    create_core_type("integer", CoreValueType::Integer)
}
pub fn i8() -> Type {
    create_core_type_with_base("integer/i8", CoreValueType::I8, integer())
}
pub fn i32() -> Type {
    create_core_type_with_base("integer/i32", CoreValueType::I32, integer())
}

pub fn text() -> Type {
    create_core_type("text", CoreValueType::Text)
}
pub fn decimal() -> Type {
    create_core_type("decimal", CoreValueType::Decimal)
}
pub fn boolean() -> Type {
    create_core_type("boolean", CoreValueType::Boolean)
}
pub fn object() -> Type {
    create_core_type("object", CoreValueType::Object)
}
pub fn array() -> Type {
    create_core_type("array", CoreValueType::Array)
}
pub fn null() -> Type {
    create_core_type("null", CoreValueType::Null)
}
