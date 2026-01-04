use crate::ast::expressions::{CreateRef, DatexExpressionData, List, Map};
use crate::ast::spanned::Spanned;
use crate::ast::type_expressions::{
    Intersection, TypeExpression, TypeExpressionData, Union,
};
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::r#type::Type;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use datex_core::ast::expressions::CallableDeclaration;
use datex_core::libs::core::CoreLibPointerId;

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
            type_to_type_expression(type_value),
        ),
        CoreValue::Callable(callable) => {
            DatexExpressionData::CallableDeclaration(CallableDeclaration {
                name: callable.name.clone(),
                kind: callable.signature.kind.clone(),
                parameters: callable
                    .signature
                    .parameter_types
                    .iter()
                    .map(|(maybe_name, ty)| {
                        (
                            maybe_name.clone().unwrap_or("_".to_string()),
                            type_to_type_expression(ty),
                        )
                    })
                    .collect(),
                rest_parameter: callable
                    .signature
                    .rest_parameter_type
                    .as_ref()
                    .map(|(maybe_name, ty)| {
                        (
                            maybe_name.clone().unwrap_or("_".to_string()),
                            type_to_type_expression(ty),
                        )
                    }),
                return_type: callable
                    .signature
                    .return_type
                    .as_ref()
                    .map(|ty| type_to_type_expression(ty)),
                yeet_type: callable
                    .signature
                    .yeet_type
                    .as_ref()
                    .map(|ty| type_to_type_expression(ty)),
                body: Box::new(
                    DatexExpressionData::NativeImplementationIndicator
                        .with_default_span(),
                ),
            })
        }
    }
}

fn type_to_type_expression(type_value: &Type) -> TypeExpression {
    match &type_value.type_definition {
        TypeDefinition::Structural(struct_type) => match struct_type {
            StructuralTypeDefinition::Integer(integer) => {
                TypeExpressionData::Integer(integer.clone()).with_default_span()
            }
            StructuralTypeDefinition::Text(text) => {
                TypeExpressionData::Text(text.0.clone()).with_default_span()
            }
            StructuralTypeDefinition::Boolean(boolean) => {
                TypeExpressionData::Boolean(boolean.0).with_default_span()
            }
            StructuralTypeDefinition::Decimal(decimal) => {
                TypeExpressionData::Decimal(decimal.clone()).with_default_span()
            }
            StructuralTypeDefinition::TypedInteger(typed_integer) => {
                TypeExpressionData::TypedInteger(typed_integer.clone())
                    .with_default_span()
            }
            StructuralTypeDefinition::TypedDecimal(typed_decimal) => {
                TypeExpressionData::TypedDecimal(typed_decimal.clone())
                    .with_default_span()
            }
            StructuralTypeDefinition::Endpoint(endpoint) => {
                TypeExpressionData::Endpoint(endpoint.clone())
                    .with_default_span()
            }
            StructuralTypeDefinition::Null => {
                TypeExpressionData::Null.with_default_span()
            }
            _ => TypeExpressionData::Text(format!(
                "[[STRUCTURAL TYPE {:?}]]",
                struct_type
            ))
            .with_default_span(),
        },
        TypeDefinition::Union(union_types) => TypeExpressionData::Union(Union(
            union_types
                .iter()
                .map(|t| type_to_type_expression(t))
                .collect::<Vec<TypeExpression>>(),
        ))
        .with_default_span(),
        TypeDefinition::Intersection(intersection_types) => {
            TypeExpressionData::Intersection(Intersection(
                intersection_types
                    .iter()
                    .map(|t| type_to_type_expression(t))
                    .collect::<Vec<TypeExpression>>(),
            ))
            .with_default_span()
        }
        TypeDefinition::Unit => TypeExpressionData::Unit.with_default_span(),
        TypeDefinition::Reference(type_reference) => {
            // try to resolve to core lib value
            if let Some(address) = &type_reference.borrow().pointer_address {
                if let Ok(core_lib_type) = CoreLibPointerId::try_from(address) {
                    TypeExpressionData::Identifier(core_lib_type.to_string())
                        .with_default_span()
                } else {
                    todo!("#651 Handle non-core-lib type references in decompiler");
                }
            } else {
                panic!("Unresolved type reference in decompiler"); // TODO #652: how to handle properly?
            }
        }
        _ => TypeExpressionData::Text(format!(
            "[[TYPE {:?}]]",
            type_value.type_definition
        ))
        .with_default_span(),
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::expressions::{DatexExpressionData, List};
    use crate::ast::spanned::Spanned;
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
