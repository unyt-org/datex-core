use crate::ast::DatexExpression;
use crate::ast::binary_operation::BinaryOperator;
use crate::libs::core::CoreLibPointerId;
use crate::runtime::Runtime;
use crate::values::core_values::array::Array;
use crate::values::core_values::object::Object;
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
) -> Result<Option<TypeNew>, TypeError> {
    Ok(match expression {
        DatexExpression::Null
        | DatexExpression::Boolean(_)
        | DatexExpression::Text(_)
        | DatexExpression::Decimal(_)
        | DatexExpression::Integer(_)
        | DatexExpression::Endpoint(_) => {
            // TODO: this unwrap asserts that try_from succeeds in all cases, but this is not yet guaranteed and tested
            Some(
                TypeNew::try_from(
                    ValueContainer::try_from(expression as &DatexExpression)
                        .unwrap(),
                )
                .unwrap(),
            )
        }
        // composite values
        DatexExpression::Object(obj) => {
            let entries = obj
                .iter_mut()
                .map(|(k, v)| {
                    let key = match k {
                        DatexExpression::Text(s) => s,
                        _ => Err(())?,
                    };
                    // TODO: is unwrap safe here?
                    let value =
                        infer_expression_type(v, runtime).unwrap().unwrap();
                    Ok((key.clone(), value.definition))
                })
                // TODO: is unwrap safe here?
                .collect::<Result<HashMap<String, ValueContainer>, ()>>()
                .unwrap();
            Some(
                TypeNew::try_from(ValueContainer::from(Object::from(entries)))
                    .unwrap(),
            )
        }
        DatexExpression::Array(arr) => {
            let entries = arr
                .iter_mut()
                .map(|v| {
                    // TODO: is unwrap safe here?
                    infer_expression_type(v, runtime)
                        .unwrap()
                        .unwrap()
                        .definition
                })
                .collect::<Vec<ValueContainer>>();
            Some(
                TypeNew::try_from(ValueContainer::from(Array::from(entries)))
                    .unwrap(),
            )
        }
        // more complex expressions
        DatexExpression::BinaryOperation(operator, lhs, rhs, cached_type) => {
            if let Some(cached) = cached_type {
                // TODO: no clone?
                Some(cached.clone())
            } else {
                infer_binary_expression_type(operator, lhs, rhs, runtime)?
            }
        }
        _ => None, // other expressions not handled yet
    })
}

fn infer_binary_expression_type(
    operator: &BinaryOperator,
    lhs: &mut Box<DatexExpression>,
    rhs: &mut Box<DatexExpression>,
    runtime: &Runtime,
) -> Result<Option<TypeNew>, TypeError> {
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
            let lhs_base_type = lhs_type.get_base_type(memory);
            let rhs_base_type = rhs_type.get_base_type(memory);

            let integer =
                memory.get_core_type_unchecked(CoreLibPointerId::Integer);
            let decimal =
                memory.get_core_type_unchecked(CoreLibPointerId::Decimal);

            // TODO: keep the type as specific as possible here? E.g. 1 + 2 -> 3, not integer
            // lhs and rhs are both integer -> result is integer
            if lhs_base_type == integer && rhs_base_type == integer {
                Ok(Some(integer))
            }
            // lhs and rhs are both decimal -> result is decimal
            else if lhs_base_type == decimal && rhs_base_type == decimal {
                Ok(Some(decimal))
            }
            // otherwise, return type error
            else {
                Err(TypeError::MismatchedOperands(
                    lhs_type.definition,
                    rhs_type.definition,
                ))
            }
        }

        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::array::Array;
    use crate::values::core_values::integer::integer::Integer;
    use datex_core::runtime::RuntimeConfig;
    use datex_core::values::core_values::decimal::decimal::Decimal;
    use datex_core::values::core_values::object::Object;

    /// Tests literal type resolution, as implemented by ValueContainer::try_from
    #[test]
    fn test_infer_literal_types() {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        assert_eq!(
            infer_expression_type(
                &mut DatexExpression::Boolean(true),
                &runtime
            )
            .unwrap()
            .unwrap(),
            TypeNew::try_from(ValueContainer::from(true)).unwrap()
        );

        assert_eq!(
            infer_expression_type(
                &mut DatexExpression::Boolean(false),
                &runtime
            )
            .unwrap()
            .unwrap(),
            TypeNew::try_from(ValueContainer::from(false)).unwrap()
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Null, &runtime)
                .unwrap()
                .unwrap(),
            TypeNew::try_from(ValueContainer::from(CoreValue::Null)).unwrap()
        );

        assert_eq!(
            infer_expression_type(
                &mut DatexExpression::Text("Hello".to_string()),
                &runtime
            )
            .unwrap()
            .unwrap(),
            TypeNew::try_from(ValueContainer::from("Hello")).unwrap()
        );

        assert_eq!(
            infer_expression_type(
                &mut DatexExpression::Decimal(Decimal::from(1.23)),
                &runtime
            )
            .unwrap()
            .unwrap(),
            TypeNew::try_from(ValueContainer::from(Decimal::from(1.23)))
                .unwrap()
        );

        assert_eq!(
            infer_expression_type(
                &mut DatexExpression::Integer(Integer::from(42)),
                &runtime
            )
            .unwrap()
            .unwrap(),
            TypeNew::try_from(ValueContainer::from(Integer::from(42))).unwrap()
        );

        assert_eq!(
            infer_expression_type(
                &mut DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3))
                ]),
                &runtime
            )
            .unwrap()
            .unwrap(),
            TypeNew::try_from(ValueContainer::from(Array::from_iter([
                ValueContainer::from(Integer::from(1)),
                ValueContainer::from(Integer::from(2)),
                ValueContainer::from(Integer::from(3))
            ])))
            .unwrap()
        );

        assert_eq!(
            infer_expression_type(
                &mut DatexExpression::Object(vec![(
                    DatexExpression::Text("a".to_string()),
                    DatexExpression::Integer(Integer::from(1))
                )]),
                &runtime
            )
            .unwrap()
            .unwrap(),
            TypeNew::try_from(ValueContainer::from(Object::from_iter(vec![(
                "a".to_string(),
                ValueContainer::from(Integer::from(1))
            )])))
            .unwrap()
        );
    }

    #[test]
    fn test_infer_binary_expression_types() {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        let integer = runtime
            .memory()
            .borrow()
            .get_core_type_unchecked(CoreLibPointerId::Integer);
        let decimal = runtime
            .memory()
            .borrow()
            .get_core_type_unchecked(CoreLibPointerId::Decimal);

        // integer - integer = integer
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Subtract,
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
            None,
        );
        assert_eq!(
            infer_expression_type(&mut expr, &runtime).unwrap().unwrap(),
            integer
        );

        // decimal - decimal = decimal
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Subtract,
            Box::new(DatexExpression::Decimal(Decimal::from(1.0))),
            Box::new(DatexExpression::Decimal(Decimal::from(2.0))),
            None,
        );
        assert_eq!(
            infer_expression_type(&mut expr, &runtime).unwrap().unwrap(),
            decimal
        );

        // integer - decimal = type error
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Subtract,
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::Decimal(Decimal::from(2.0))),
            None,
        );
        assert!(infer_expression_type(&mut expr, &runtime).is_err());
    }

    #[test]
    fn test_infer_nested_binary_expression_types() {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        let integer = runtime
            .memory()
            .borrow()
            .get_core_type_unchecked(CoreLibPointerId::Integer);

        // (1 - 2) - 3 -> integer
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Subtract,
            Box::new(DatexExpression::BinaryOperation(
                BinaryOperator::Subtract,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None,
            )),
            Box::new(DatexExpression::Integer(Integer::from(3))),
            None,
        );
        assert_eq!(
            infer_expression_type(&mut expr, &runtime).unwrap().unwrap(),
            integer
        );

        // {a: 1 - 2} -> {a: integer}
        let mut expr = DatexExpression::Object(vec![(
            DatexExpression::Text("a".to_string()),
            DatexExpression::BinaryOperation(
                BinaryOperator::Subtract,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None,
            ),
        )]);
        assert_eq!(
            infer_expression_type(&mut expr, &runtime).unwrap().unwrap(),
            TypeNew::try_from(ValueContainer::from(Object::from_iter(vec![(
                "a".to_string(),
                integer.definition.clone()
            )])))
            .unwrap()
        );

        // [1, 2 - 3] -> [1, integer]
        let mut expr = DatexExpression::Array(vec![
            DatexExpression::Integer(Integer::from(1)),
            DatexExpression::BinaryOperation(
                BinaryOperator::Subtract,
                Box::new(DatexExpression::Integer(Integer::from(2))),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None,
            ),
        ]);
        assert_eq!(
            infer_expression_type(&mut expr, &runtime).unwrap().unwrap(),
            TypeNew::try_from(ValueContainer::from(Array::from_iter(vec![
                ValueContainer::from(Integer::from(1)),
                integer.definition.clone()
            ])))
            .unwrap()
        );
    }
}
