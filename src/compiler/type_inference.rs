use crate::ast::DatexExpression;
use crate::values::core_value::CoreValue;
use crate::values::value_container::ValueContainer;

pub enum TypeError {
    MismatchedOperands(ValueContainer, ValueContainer),
}

/// Infers the type of an expression as precisely as possible.
/// Uses cached type information if available.
fn infer_expression_type(
    expression: &mut DatexExpression,
) -> Option<ValueContainer> {
    match expression {
        DatexExpression::Null |
        DatexExpression::Boolean(_) |
        DatexExpression::Text(_) |
        DatexExpression::Decimal(_) |
        DatexExpression::Integer(_) |
        DatexExpression::Endpoint(_) |
        DatexExpression::Array(_) |
        DatexExpression::Object(_) => {
            // TODO: this unwrap asserts that try_from succeeds in all cases, but this is not yet guaranteed and tested
            Some(ValueContainer::try_from(expression as &DatexExpression).unwrap())
        }
        // more complex expressions
        DatexExpression::BinaryOperation(operator, lhs, rhs, cached_type) => {
            if let Some(cached) = cached_type {
                return Some(cached.clone());
            }

            let lhs_type = infer_expression_type(lhs)?;
            let rhs_type = infer_expression_type(rhs)?;
            todo!()
        }
        _ => None, // other expressions not handled yet
    }
}


#[cfg(test)]
mod tests {
    use datex_core::values::core_values::decimal::decimal::Decimal;
    use datex_core::values::core_values::object::Object;
    use crate::values::core_values::array::Array;
    use crate::values::core_values::integer::integer::Integer;
    use super::*;

    /// Tests literal type resolution, as implemented by ValueContainer::try_from
    #[test]
    fn test_infer_literal_types() {
        assert_eq!(
            infer_expression_type(&mut DatexExpression::Boolean(true)),
            Some(ValueContainer::from(true))
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Boolean(false)),
            Some(ValueContainer::from(false))
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Null),
            Some(ValueContainer::from(CoreValue::Null))
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Text("Hello".to_string())),
            Some(ValueContainer::from("Hello".to_string()))
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Decimal(Decimal::from(1.23))),
            Some(ValueContainer::from(Decimal::from(1.23)))
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Integer(Integer::from(42))),
            Some(ValueContainer::from(Integer::from(42)))
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Array(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
                DatexExpression::Integer(Integer::from(3))
            ])),
            Some(ValueContainer::from(Array::from_iter([
                ValueContainer::from(Integer::from(1)),
                ValueContainer::from(Integer::from(2)),
                ValueContainer::from(Integer::from(3))
            ])))
        );

        assert_eq!(
            infer_expression_type(&mut DatexExpression::Object(vec![
                (
                    DatexExpression::Text("a".to_string()),
                    DatexExpression::Integer(Integer::from(1))
                )
            ])),
            Some(ValueContainer::from(Object::from_iter(vec![
                (
                    "a".to_string(),
                    ValueContainer::from(Integer::from(1))
                )
            ])))
        );
    }
}
