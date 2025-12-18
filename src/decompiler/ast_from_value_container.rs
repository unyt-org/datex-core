use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{
    CreateRef, DatexExpressionData, List, Map,
};
use crate::ast::structs::r#type::TypeExpressionData;
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

impl From<&ValueContainer> for DatexExpressionData {
    /// Converts a ValueContainer into a DatexExpression AST.
    /// This AST can then be further processed or decompiled into human-readable DATEX code.
    fn from(value: &ValueContainer) -> Self {
        match value {
            ValueContainer::Value(value) => value_to_datex_expression(value),
            ValueContainer::Reference(reference) => {
                DatexExpressionData::CreateRef(CreateRef {
                    mutability: reference.mutability(),
                    expression: Box::new(
                        DatexExpressionData::from(&reference.value_container())
                            .with_default_span(),
                    ),
                })
            }
        }
    }
}

fn value_to_datex_expression(value: &Value) -> DatexExpressionData {
    match &value.inner {
        CoreValue::Integer(integer) => {
            DatexExpressionData::Integer(integer.clone())
        }
        CoreValue::TypedInteger(typed_integer) => {
            DatexExpressionData::TypedInteger(typed_integer.clone())
        }
        CoreValue::Decimal(decimal) => {
            DatexExpressionData::Decimal(decimal.clone())
        }
        CoreValue::TypedDecimal(typed_decimal) => {
            DatexExpressionData::TypedDecimal(typed_decimal.clone())
        }
        CoreValue::Boolean(boolean) => DatexExpressionData::Boolean(boolean.0),
        CoreValue::Text(text) => DatexExpressionData::Text(text.0.clone()),
        CoreValue::RangeDefinition(range) => {
            DatexExpressionData::RangeDefinition(range.clone())
        }
        CoreValue::Endpoint(endpoint) => {
            DatexExpressionData::Endpoint(endpoint.clone())
        }
        CoreValue::Null => DatexExpressionData::Null,
        CoreValue::List(list) => DatexExpressionData::List(List::new(
            list.into_iter()
                .map(DatexExpressionData::from)
                .map(|data| data.with_default_span())
                .collect(),
        )),
        CoreValue::Map(map) => DatexExpressionData::Map(Map::new(
            map.into_iter()
                .map(|(key, value)| {
                    (
                        DatexExpressionData::from(&ValueContainer::from(key))
                            .with_default_span(),
                        DatexExpressionData::from(value).with_default_span(),
                    )
                })
                .collect(),
        )),
        CoreValue::Type(type_value) => DatexExpressionData::TypeExpression(
            match &type_value.type_definition {
                TypeDefinition::Structural(struct_type) => match struct_type {
                    StructuralTypeDefinition::Integer(integer) => {
                        TypeExpressionData::Integer(integer.clone())
                            .with_default_span()
                    }
                    _ => core::todo!("#416 Undescribed by author."),
                },
                _ => core::todo!("#417 Undescribed by author."),
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{DatexExpressionData, List};
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::integer::Integer;
    use crate::values::core_values::integer::typed_integer::TypedInteger;
    use crate::values::value::Value;
    use crate::values::value_container::ValueContainer;

    #[test]
    fn test_integer_to_ast() {
        let value = ValueContainer::from(Integer::from(42));
        let ast = DatexExpressionData::from(&value);
        assert_eq!(ast, DatexExpressionData::Integer(Integer::from(42)));
    }

    #[test]
    fn test_typed_integer_to_ast() {
        let value = ValueContainer::from(TypedInteger::from(42i8));
        let ast = DatexExpressionData::from(&value);
        assert_eq!(
            ast,
            DatexExpressionData::TypedInteger(TypedInteger::from(42i8))
        );
    }

    #[test]
    fn test_decimal_to_ast() {
        let value = ValueContainer::from(Decimal::from(1.23));
        let ast = DatexExpressionData::from(&value);
        assert_eq!(ast, DatexExpressionData::Decimal(Decimal::from(1.23)));
    }

    #[test]
    fn test_typed_decimal_to_ast() {
        let value = ValueContainer::from(TypedDecimal::from(2.71f32));
        let ast = DatexExpressionData::from(&value);
        assert_eq!(
            ast,
            DatexExpressionData::TypedDecimal(TypedDecimal::from(2.71f32))
        );
    }

    #[test]
    fn test_boolean_to_ast() {
        let value = ValueContainer::from(true);
        let ast = DatexExpressionData::from(&value);
        assert_eq!(ast, DatexExpressionData::Boolean(true));
    }

    #[test]
    fn test_text_to_ast() {
        let value = ValueContainer::from("Hello, World!".to_string());
        let ast = DatexExpressionData::from(&value);
        assert_eq!(ast, DatexExpressionData::Text("Hello, World!".to_string()));
    }

    #[test]
    fn test_null_to_ast() {
        let value = ValueContainer::Value(Value::null());
        let ast = DatexExpressionData::from(&value);
        assert_eq!(ast, DatexExpressionData::Null);
    }

    #[test]
    fn test_list_to_ast() {
        let value = ValueContainer::from(vec![
            Integer::from(1),
            Integer::from(2),
            Integer::from(3),
        ]);
        let ast = DatexExpressionData::from(&value);
        assert_eq!(
            ast,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
            ]))
        );
    }
}
