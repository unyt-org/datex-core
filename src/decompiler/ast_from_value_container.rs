use crate::ast::DatexExpression;
use crate::values::core_value::CoreValue;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

impl From<ValueContainer> for DatexExpression {
    /// Converts a ValueContainer into a DatexExpression AST.
    /// This AST can then be further processed or decompiled into human-readable DATEX code.
    fn from(value: ValueContainer) -> Self {
        match value {
            ValueContainer::Value(Value { inner, actual_type}) => {
                match inner {
                    CoreValue::Integer(integer) => DatexExpression::Integer(integer),
                    CoreValue::TypedInteger(typed_integer) => DatexExpression::TypedInteger(typed_integer),
                    CoreValue::Decimal(decimal) => DatexExpression::Decimal(decimal),
                    CoreValue::TypedDecimal(typed_decimal) => DatexExpression::TypedDecimal(typed_decimal),
                    CoreValue::Boolean(boolean) => DatexExpression::Boolean(boolean.0),
                    CoreValue::Text(text) => DatexExpression::Text(text.0),
                    CoreValue::Null => DatexExpression::Null,
                    CoreValue::Array(array) => DatexExpression::Array(array.into_iter().map(DatexExpression::from).collect()),
                    _ => todo!()
                }
            }
            ValueContainer::Reference(reference) => todo!()
        }
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
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::Integer(Integer::from(42)));
    }

    #[test]
    fn test_typed_integer_to_ast() {
        let value = ValueContainer::from(TypedInteger::from(42i8));
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::TypedInteger(TypedInteger::from(42i8)));
    }

    #[test]
    fn test_decimal_to_ast() {
        let value = ValueContainer::from(Decimal::from(1.23));
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::Decimal(Decimal::from(1.23)));
    }

    #[test]
    fn test_typed_decimal_to_ast() {
        let value = ValueContainer::from(TypedDecimal::from(2.71f32));
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::TypedDecimal(TypedDecimal::from(2.71f32)));
    }

    #[test]
    fn test_boolean_to_ast() {
        let value = ValueContainer::from(true);
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::Boolean(true));
    }

    #[test]
    fn test_text_to_ast() {
        let value = ValueContainer::from("Hello, World!".to_string());
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::Text("Hello, World!".to_string()));
    }

    #[test]
    fn test_null_to_ast() {
        let value = ValueContainer::Value(Value::null());
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::Null);
    }

    #[test]
    fn test_array_to_ast() {
        let value = ValueContainer::from(vec![
            Integer::from(1),
            Integer::from(2),
            Integer::from(3),
        ]);
        let ast = DatexExpression::from(value);
        assert_eq!(ast, DatexExpression::Array(vec![
            DatexExpression::Integer(Integer::from(1)),
            DatexExpression::Integer(Integer::from(2)),
            DatexExpression::Integer(Integer::from(3)),
        ]));
    }
}