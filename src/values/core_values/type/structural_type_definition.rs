use crate::values::core_value::CoreValue;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::text::Text;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::type_container::TypeContainer;
use crate::values::value_container::ValueContainer;
use datex_core::values::core_values::endpoint::Endpoint;
use datex_core::values::core_values::integer::integer::Integer;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum StructuralTypeDefinition {
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
    Map(Box<(TypeContainer, TypeContainer)>),
}

macro_rules! impl_from_typed_int {
    ($($t:ty),*) => {
        $(
            impl From<$t> for StructuralTypeDefinition {
                fn from(value: $t) -> Self {
                    StructuralTypeDefinition::TypedInteger(TypedInteger::from(value))
                }
            }
        )*
    }
}
impl_from_typed_int!(u8, u16, u32, u64, i8, i16, i32, i64);

impl From<String> for StructuralTypeDefinition {
    fn from(value: String) -> Self {
        StructuralTypeDefinition::Text(Text::from(value))
    }
}
impl From<&str> for StructuralTypeDefinition {
    fn from(value: &str) -> Self {
        StructuralTypeDefinition::Text(Text::from(value))
    }
}

impl From<Integer> for StructuralTypeDefinition {
    fn from(value: Integer) -> Self {
        StructuralTypeDefinition::Integer(value)
    }
}
impl From<TypedInteger> for StructuralTypeDefinition {
    fn from(value: TypedInteger) -> Self {
        StructuralTypeDefinition::TypedInteger(value)
    }
}

impl From<TypedDecimal> for StructuralTypeDefinition {
    fn from(value: TypedDecimal) -> Self {
        StructuralTypeDefinition::TypedDecimal(value)
    }
}

impl From<Decimal> for StructuralTypeDefinition {
    fn from(value: Decimal) -> Self {
        StructuralTypeDefinition::Decimal(value)
    }
}

impl From<Text> for StructuralTypeDefinition {
    fn from(value: Text) -> Self {
        StructuralTypeDefinition::Text(value)
    }
}
impl From<Boolean> for StructuralTypeDefinition {
    fn from(value: Boolean) -> Self {
        StructuralTypeDefinition::Boolean(value)
    }
}

impl From<Endpoint> for StructuralTypeDefinition {
    fn from(value: Endpoint) -> Self {
        StructuralTypeDefinition::Endpoint(value)
    }
}

impl StructuralTypeDefinition {
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
            (StructuralTypeDefinition::Integer(a), CoreValue::Integer(b)) => {
                a == b
            }
            (
                StructuralTypeDefinition::TypedInteger(a),
                CoreValue::TypedInteger(b),
            ) => a == b,
            (StructuralTypeDefinition::Decimal(a), CoreValue::Decimal(b)) => {
                a == b
            }
            (
                StructuralTypeDefinition::TypedDecimal(a),
                CoreValue::TypedDecimal(b),
            ) => a == b,
            (StructuralTypeDefinition::Text(a), CoreValue::Text(b)) => a == b,
            (StructuralTypeDefinition::Boolean(a), CoreValue::Boolean(b)) => {
                a == b
            }
            (StructuralTypeDefinition::Endpoint(a), CoreValue::Endpoint(b)) => {
                a == b
            }
            (StructuralTypeDefinition::Null, CoreValue::Null) => true,

            (
                StructuralTypeDefinition::List(box elem_type),
                CoreValue::List(list),
            ) => list.into_iter().all(|item| elem_type.value_matches(item)),
            _ => unimplemented!("handle complex structural type matching"),
        }
    }
}

impl StructuralEq for StructuralTypeDefinition {
    fn structural_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Display for StructuralTypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructuralTypeDefinition::Integer(integer) => {
                write!(f, "{}", integer)
            }
            StructuralTypeDefinition::TypedInteger(typed_integer) => {
                write!(f, "{}", typed_integer)
            }
            StructuralTypeDefinition::Decimal(decimal) => {
                write!(f, "{}", decimal)
            }
            StructuralTypeDefinition::TypedDecimal(typed_decimal) => {
                write!(f, "{}", typed_decimal)
            }
            StructuralTypeDefinition::Text(text) => write!(f, "{}", text),
            StructuralTypeDefinition::Boolean(boolean) => {
                write!(f, "{}", boolean)
            }
            StructuralTypeDefinition::Endpoint(endpoint) => {
                write!(f, "{}", endpoint)
            }
            StructuralTypeDefinition::Null => write!(f, "null"),
            StructuralTypeDefinition::Array(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "[{}]", types_str.join(", "))
            }
            StructuralTypeDefinition::Tuple(elements) => {
                let elements_str: Vec<String> = elements
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "({})", elements_str.join(", "))
            }
            StructuralTypeDefinition::List(element_type) => {
                write!(f, "List<{}>", element_type)
            }
            StructuralTypeDefinition::Map(box (key_type, value_type)) => {
                write!(f, "Map<{}, {}>", key_type, value_type)
            }
            StructuralTypeDefinition::Struct(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", fields_str.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::integer::integer::Integer;
    use crate::values::core_values::text::Text;
    use crate::values::core_values::r#type::Type;
    use crate::values::core_values::r#type::structural_type_definition::StructuralTypeDefinition;
    use crate::values::value_container::ValueContainer;
    #[test]
    fn test_structural_type_display() {
        let int_type = StructuralTypeDefinition::Integer(Integer::from(42));
        assert_eq!(int_type.to_string(), "42");

        let text_type = StructuralTypeDefinition::Text(Text::from("Hello"));
        assert_eq!(text_type.to_string(), r#""Hello""#);

        let array_type = StructuralTypeDefinition::Array(vec![
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                1,
            )))
            .into(),
            Type::structural(StructuralTypeDefinition::Text(Text::from(
                "World",
            )))
            .into(),
        ]);
        assert_eq!(array_type.to_string(), r#"[1, "World"]"#);

        let struct_type = StructuralTypeDefinition::Struct(vec![
            ("id".to_string(), Type::structural(int_type.clone()).into()),
            (
                "name".to_string(),
                Type::structural(text_type.clone()).into(),
            ),
        ]);
        assert_eq!(struct_type.to_string(), r#"{id: 42, name: "Hello"}"#);

        let list_type = StructuralTypeDefinition::List(Box::new(
            Type::structural(StructuralTypeDefinition::Text(Text::from(
                "Item",
            )))
            .into(),
        ));
        assert_eq!(list_type.to_string(), r#"List<"Item">"#);

        let map_type = StructuralTypeDefinition::Map(Box::new((
            Type::structural(StructuralTypeDefinition::Text(Text::from("Key")))
                .into(),
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                100,
            )))
            .into(),
        )));
        assert_eq!(map_type.to_string(), r#"Map<"Key", 100>"#);
    }


    #[test]
    fn test_value_matching() {
        let int_type = StructuralTypeDefinition::Integer(Integer::from(42));
        let int_value =
            ValueContainer::from(CoreValue::Integer(Integer::from(42)));
        assert!(int_type.value_matches(&int_value));

        let text_type = StructuralTypeDefinition::Text(Text::from("Hello"));
        let text_value =
            ValueContainer::from(CoreValue::Text(Text::from("Hello")));
        assert!(text_type.value_matches(&text_value));

        let list_type = StructuralTypeDefinition::List(Box::new(
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                1,
            )))
            .into(),
        ));
        let list_value = ValueContainer::from(vec![
            CoreValue::Integer(Integer::from(1)),
            CoreValue::Integer(Integer::from(1)),
        ]);
        assert!(list_type.value_matches(&list_value));
    }
}
