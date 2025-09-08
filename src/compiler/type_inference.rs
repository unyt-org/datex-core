use crate::ast::DatexExpression;
use crate::ast::binary_operation::BinaryOperator;
use crate::libs::core::CoreLibPointerId;
use crate::runtime::Runtime;
use crate::values::core_values::list::List;
use crate::values::core_values::map::Map;
use crate::values::core_values::r#type::Type;
use crate::values::core_values::r#type::structural_type::StructuralType;
use crate::values::type_container::TypeContainer;
use crate::values::value_container::ValueContainer;
use std::collections::HashMap;

#[derive(Debug)]
pub enum TypeError {
    MismatchedOperands(ValueContainer, ValueContainer),
}

/// Infers the type of an expression as precisely as possible.
/// Uses cached type information if available.
fn infer_expression_type(
    expression: &mut DatexExpression,
    runtime: &Runtime,
) -> Result<Option<TypeContainer>, TypeError> {
    Ok(match expression {
        DatexExpression::Null
        | DatexExpression::Boolean(_)
        | DatexExpression::Text(_)
        | DatexExpression::Decimal(_)
        | DatexExpression::Integer(_)
        | DatexExpression::Endpoint(_) => {
            // TODO: this unwrap asserts that try_from succeeds in all cases, but this is not yet guaranteed and tested
            let value = Type::try_from(expression as &DatexExpression).unwrap();
            Some(TypeContainer::Type(value))
        }
        // composite values
        DatexExpression::Map(map) => {
            todo!()
            // let entries = map
            //     .iter_mut()
            //     .map(|(k, v)| {
            //         // TODO: is unwrap safe here?
            //         let value =
            //             infer_expression_type(v, runtime).unwrap().unwrap();
            //         let key =
            //             infer_expression_type(k, runtime).unwrap().unwrap();
            //         Ok((key, value))
            //     })
            //     // TODO: is unwrap safe here?
            //     .collect::<Result<Vec<(_, _)>, ()>>()
            //     .unwrap();
            // Some(TypeContainer::Type(Type::structural(
            //     StructuralType::Map(entries),
            // )))
        }
        DatexExpression::Struct(structure) => {
            let entries = structure
                .iter_mut()
                .map(|(k, v)| {
                    // TODO: is unwrap safe here?
                    let value =
                        infer_expression_type(v, runtime).unwrap().unwrap();
                    Ok((k.clone(), value))
                })
                // TODO: is unwrap safe here?
                .collect::<Result<Vec<(_, _)>, ()>>()
                .unwrap();
            Some(TypeContainer::Type(Type::structural(
                StructuralType::Struct(entries),
            )))
        }
        DatexExpression::Array(arr) => {
            let entries = arr
                .iter_mut()
                .map(|v| {
                    // TODO: is unwrap safe here?
                    infer_expression_type(v, runtime).unwrap().unwrap()
                })
                .collect::<Vec<_>>();
            Some(TypeContainer::Type(Type::structural(
                StructuralType::Array(entries),
            )))
        }
        // more complex expressions
        DatexExpression::BinaryOperation(operator, lhs, rhs, cached_type) => {
            // if let Some(cached) = cached_type {
            //     // TODO: no clone?
            //     Some(cached.clone())
            // } else {
            infer_binary_expression_type(operator, lhs, rhs, runtime)?
            // }
        }
        _ => None, // other expressions not handled yet
    })
}

fn infer_binary_expression_type(
    operator: &BinaryOperator,
    lhs: &mut Box<DatexExpression>,
    rhs: &mut Box<DatexExpression>,
    runtime: &Runtime,
) -> Result<Option<TypeContainer>, TypeError> {
    let lhs_type = infer_expression_type(lhs, runtime)?;
    let rhs_type = infer_expression_type(rhs, runtime)?;

    if lhs_type.is_none() || rhs_type.is_none() {
        // TODO: handle expressions that return "void" here
        return Ok(None);
    }
    let lhs_type = lhs_type.unwrap();
    let rhs_type = rhs_type.unwrap();

    let memory = &*runtime.memory().borrow();

    match operator {
        // numeric-type only operations
        BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide => {
            let lhs_base_type = lhs_type.base_type();
            let rhs_base_type = rhs_type.base_type();

            todo!("handle core type");

            // let integer =
            //     memory.get_core_type_unchecked(CoreLibPointerId::Integer);
            // let decimal =
            //     memory.get_core_type_unchecked(CoreLibPointerId::Decimal);

            // // TODO: keep the type as specific as possible here? E.g. 1 + 2 -> 3, not integer
            // // lhs and rhs are both integer -> result is integer
            // if lhs_base_type == integer && rhs_base_type == integer {
            //     Ok(Some(integer))
            // }
            // // lhs and rhs are both decimal -> result is decimal
            // else if lhs_base_type == decimal && rhs_base_type == decimal {
            //     Ok(Some(decimal))
            // }
            // // otherwise, return type error
            // else {
            //     Err(TypeError::MismatchedOperands(
            //         lhs_type.definition,
            //         rhs_type.definition,
            //     ))
            // }
        }

        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::list::List;
    use crate::values::core_values::integer::integer::Integer;
    use datex_core::runtime::RuntimeConfig;
    use datex_core::values::core_values::decimal::decimal::Decimal;
    use datex_core::values::core_values::map::Map;

    fn infer_get_type(expr: &mut DatexExpression, runtime: &Runtime) -> Type {
        infer_expression_type(expr, runtime)
            .unwrap()
            .and_then(|tc| tc.as_type())
            .expect("TypeContainer should contain a Type")
    }

    /// Tests literal type resolution, as implemented by ValueContainer::try_from
    #[test]
    fn test_infer_literal_types() {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        assert_eq!(
            infer_get_type(&mut DatexExpression::Boolean(true), &runtime),
            Type::from(CoreValue::from(true))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpression::Boolean(false), &runtime),
            Type::from(CoreValue::from(false))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpression::Null, &runtime),
            Type::from(CoreValue::Null)
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Text("Hello".to_string()),
                &runtime
            ),
            Type::from(CoreValue::from("Hello".to_string()))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Decimal(Decimal::from(1.23)),
                &runtime
            ),
            Type::from(CoreValue::from(Decimal::from(1.23)))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Integer(Integer::from(42)),
                &runtime
            ),
            Type::from(CoreValue::from(Integer::from(42)))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3))
                ]),
                &runtime
            ),
            Type::from(CoreValue::from(List::from_iter([
                ValueContainer::from(Integer::from(1)),
                ValueContainer::from(Integer::from(2)),
                ValueContainer::from(Integer::from(3))
            ])))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Struct(vec![(
                    "a".to_string(),
                    DatexExpression::Integer(Integer::from(1))
                )]),
                &runtime
            ),
            Type::from(CoreValue::from(Map::from_iter(vec![(
                "a".to_string(),
                ValueContainer::from(Integer::from(1))
            )])))
        );
    }

    #[test]
    #[ignore = "Disabled due to type implmementation changes"]
    fn test_infer_binary_expression_types() {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        let integer = runtime
            .memory()
            .borrow()
            .get_core_type_unchecked(CoreLibPointerId::Array);
        let decimal = runtime
            .memory()
            .borrow()
            .get_core_type_unchecked(CoreLibPointerId::Array);

        // integer - integer = integer
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Subtract,
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
            None,
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Text("Hello".to_string()),
                &runtime
            ),
            Type::from(CoreValue::from("Hello".to_string()))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Decimal(Decimal::from(1.23)),
                &runtime
            ),
            Type::from(CoreValue::from(Decimal::from(1.23)))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Integer(Integer::from(42)),
                &runtime
            ),
            Type::from(CoreValue::from(Integer::from(42)))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3))
                ]),
                &runtime
            ),
            Type::from(CoreValue::from(List::from_iter([
                ValueContainer::from(Integer::from(1)),
                ValueContainer::from(Integer::from(2)),
                ValueContainer::from(Integer::from(3))
            ])))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Struct(vec![(
                    "a".to_string(),
                    DatexExpression::Integer(Integer::from(1))
                )]),
                &runtime
            ),
            Type::from(CoreValue::from(Map::from_iter(vec![(
                "a".to_string(),
                ValueContainer::from(Integer::from(1))
            )])))
        );
    }
}
