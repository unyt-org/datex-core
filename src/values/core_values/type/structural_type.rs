use crate::values::core_value::CoreValue;
use crate::values::core_value_trait::CoreValueTrait;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::text::Text;
use crate::values::core_values::r#type::Type;
use crate::values::core_values::r#type::definition::TypeDefinition;
use crate::values::reference::{Reference, ReferenceMutability};
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::type_container::TypeContainer;
use crate::values::value_container::ValueContainer;
use datex_core::values::core_values::endpoint::Endpoint;
use datex_core::values::core_values::integer::integer::Integer;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum StructuralType {
    Integer(Integer),
    TypedInteger(TypedInteger),
    Decimal(Decimal),
    TypedDecimal(TypedDecimal),
    Text(Text),
    Boolean(Boolean),
    Endpoint(Endpoint),
    Null,
    Array(Vec<TypeContainer>),
    List(Box<TypeContainer>),
    Tuple(Vec<(TypeContainer, TypeContainer)>),
    Struct(Vec<(String, TypeContainer)>),
    Map(Box<(TypeContainer, TypeContainer)>)
}

impl From<Integer> for StructuralType {
    fn from(value: Integer) -> Self {
        StructuralType::Integer(value)
    }
}
impl From<TypedInteger> for StructuralType {
    fn from(value: TypedInteger) -> Self {
        StructuralType::TypedInteger(value)
    }
}

impl From<TypedDecimal> for StructuralType {
    fn from(value: TypedDecimal) -> Self {
        StructuralType::TypedDecimal(value)
    }
}

impl From<Decimal> for StructuralType {
    fn from(value: Decimal) -> Self {
        StructuralType::Decimal(value)
    }
}

impl From<Text> for StructuralType {
    fn from(value: Text) -> Self {
        StructuralType::Text(value)
    }
}
impl From<Boolean> for StructuralType {
    fn from(value: Boolean) -> Self {
        StructuralType::Boolean(value)
    }
}

impl From<Endpoint> for StructuralType {
    fn from(value: Endpoint) -> Self {
        StructuralType::Endpoint(value)
    }
}

impl StructuralType {
    /// Matches a value against self
    /// Returns true if all possible realizations of the value match the type
    /// Examples:
    /// 1 matches 1 -> true
    /// 1 matches 2 -> false
    /// 1 matches 1 | 2 -> true
    /// 1 | 2 matches integer -> true
    /// integer matches 1 | 2 -> false
    pub fn value_matches(&self, value: &ValueContainer) -> bool {
        match (self, &value.to_value().borrow().inner) {
            (StructuralType::Integer(a), CoreValue::Integer(b)) => a == b,
            (StructuralType::TypedInteger(a), CoreValue::TypedInteger(b)) => {
                a == b
            }
            (StructuralType::Decimal(a), CoreValue::Decimal(b)) => a == b,
            (StructuralType::TypedDecimal(a), CoreValue::TypedDecimal(b)) => {
                a == b
            }
            (StructuralType::Text(a), CoreValue::Text(b)) => a == b,
            (StructuralType::Boolean(a), CoreValue::Boolean(b)) => a == b,
            (StructuralType::Endpoint(a), CoreValue::Endpoint(b)) => a == b,
            (StructuralType::Null, CoreValue::Null) => true,
            _ => unimplemented!("handle complex structural type matching"),
        }
    }
}

impl StructuralEq for StructuralType {
    fn structural_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Display for StructuralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructuralType::Integer(integer) => write!(f, "{}", integer),
            StructuralType::TypedInteger(typed_integer) => {
                write!(f, "{}", typed_integer)
            }
            StructuralType::Decimal(decimal) => write!(f, "{}", decimal),
            StructuralType::TypedDecimal(typed_decimal) => {
                write!(f, "{}", typed_decimal)
            }
            StructuralType::Text(text) => write!(f, "{}", text),
            StructuralType::Boolean(boolean) => write!(f, "{}", boolean),
            StructuralType::Endpoint(endpoint) => write!(f, "{}", endpoint),
            StructuralType::Null => write!(f, "null"),
            StructuralType::Array(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "[{}]", types_str.join(", "))
            }
            StructuralType::Tuple(elements) => {
                let elements_str: Vec<String> = elements
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "({})", elements_str.join(", "))
            }
            StructuralType::List(element_type) => {
                write!(f, "List<{}>", element_type)
            }
            StructuralType::Map(box (key_type, value_type)) => {
                write!(f, "Map<{}, {}>", key_type, value_type)
            }
            StructuralType::Struct(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", fields_str.join(", "))
            }
        }
    }
}
