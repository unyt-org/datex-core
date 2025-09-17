use crate::ast::DatexExpression;
use crate::values::core_value::CoreValue;
use crate::values::reference::ReferenceMutability;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

impl From<&ValueContainer> for DatexExpression {
    /// Converts a ValueContainer into a DatexExpression AST.
    /// This AST can then be further processed or decompiled into human-readable DATEX code.
    fn from(value: &ValueContainer) -> Self {
        match value {
            ValueContainer::Value(value) => value_to_datex_expression(value),
            ValueContainer::Reference(reference) => {
                match reference.mutability() {
                    ReferenceMutability::Mutable => DatexExpression::RefMut(
                        Box::new(DatexExpression::from(&reference.value_container())),
                    ),
                    ReferenceMutability::Immutable => DatexExpression::Ref(
                        Box::new(DatexExpression::from(&reference.value_container()))
                    ),
                    ReferenceMutability::Final => DatexExpression::RefFinal(
                        Box::new(DatexExpression::from(&reference.value_container()))
                    )
                }
            }
        }
    }
}

fn value_to_datex_expression(value: &Value) -> DatexExpression {
    match &value.inner {
        CoreValue::Integer(integer) => {
            DatexExpression::Integer(integer.clone())
        }
        CoreValue::TypedInteger(typed_integer) => {
            DatexExpression::TypedInteger(typed_integer.clone())
        }
        CoreValue::Decimal(decimal) => {
            DatexExpression::Decimal(decimal.clone())
        }
        CoreValue::TypedDecimal(typed_decimal) => {
            DatexExpression::TypedDecimal(typed_decimal.clone())
        }
        CoreValue::Boolean(boolean) => DatexExpression::Boolean(boolean.0),
        CoreValue::Text(text) => DatexExpression::Text(text.0.clone()),
        CoreValue::Endpoint(endpoint) => {
            DatexExpression::Endpoint(endpoint.clone())
        }
        CoreValue::Null => DatexExpression::Null,
        CoreValue::List(list) => DatexExpression::List(
            list.into_iter().map(DatexExpression::from).collect(),
        ),
        CoreValue::Array(list) => DatexExpression::Array(
            list.into_iter().map(DatexExpression::from).collect(),
        ),
        CoreValue::Map(map) => DatexExpression::Map(
            map.into_iter()
                .map(|(key, value)| {
                    (DatexExpression::from(key), DatexExpression::from(value))
                })
                .collect(),
        ),
        CoreValue::Struct(structure) => DatexExpression::Struct(
            structure
                .iter()
                .map(|(key, value)| (key, DatexExpression::from(value)))
                .collect(),
        ),
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::DatexExpression;
    use crate::values::core_values::decimal::decimal::Decimal;
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::integer::integer::Integer;
    use crate::values::core_values::integer::typed_integer::TypedInteger;
    use crate::values::value::Value;
    use crate::values::value_container::ValueContainer;

    #[test]
    fn test_integer_to_ast() {
        let value = ValueContainer::from(Integer::from(42));
        let ast = DatexExpression::from(&value);
        assert_eq!(ast, DatexExpression::Integer(Integer::from(42)));
    }

    #[test]
    fn test_typed_integer_to_ast() {
        let value = ValueContainer::from(TypedInteger::from(42i8));
        let ast = DatexExpression::from(&value);
        assert_eq!(
            ast,
            DatexExpression::TypedInteger(TypedInteger::from(42i8))
        );
    }

    #[test]
    fn test_decimal_to_ast() {
        let value = ValueContainer::from(Decimal::from(1.23));
        let ast = DatexExpression::from(&value);
        assert_eq!(ast, DatexExpression::Decimal(Decimal::from(1.23)));
    }

    #[test]
    fn test_typed_decimal_to_ast() {
        let value = ValueContainer::from(TypedDecimal::from(2.71f32));
        let ast = DatexExpression::from(&value);
        assert_eq!(
            ast,
            DatexExpression::TypedDecimal(TypedDecimal::from(2.71f32))
        );
    }

    #[test]
    fn test_boolean_to_ast() {
        let value = ValueContainer::from(true);
        let ast = DatexExpression::from(&value);
        assert_eq!(ast, DatexExpression::Boolean(true));
    }

    #[test]
    fn test_text_to_ast() {
        let value = ValueContainer::from("Hello, World!".to_string());
        let ast = DatexExpression::from(&value);
        assert_eq!(ast, DatexExpression::Text("Hello, World!".to_string()));
    }

    #[test]
    fn test_null_to_ast() {
        let value = ValueContainer::Value(Value::null());
        let ast = DatexExpression::from(&value);
        assert_eq!(ast, DatexExpression::Null);
    }

    #[test]
    fn test_array_to_ast() {
        let value = ValueContainer::from(vec![
            Integer::from(1),
            Integer::from(2),
            Integer::from(3),
        ]);
        let ast = DatexExpression::from(&value);
        assert_eq!(
            ast,
            DatexExpression::Array(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
                DatexExpression::Integer(Integer::from(3)),
            ])
        );
    }
}
