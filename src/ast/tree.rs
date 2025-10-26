use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::BinaryOperator;
use crate::ast::binding::VariableId;
use crate::ast::chain::ApplyOperation;
use crate::ast::comparison_operation::ComparisonOperator;
use crate::ast::unary_operation::{ArithmeticUnaryOperator, UnaryOperator};
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use std::fmt::Display;
use std::ops::Neg;

pub use chumsky::prelude::SimpleSpan;

// TODO #470: implement Visitable for all expressions with children
pub(crate) trait Spanned: Sized {
    type Output;
    fn with_span(self, span: SimpleSpan) -> Self::Output;
    fn with_default_span(self) -> Self::Output;
}

impl Spanned for DatexExpressionData {
    type Output = DatexExpression;

    fn with_span(self, span: SimpleSpan) -> Self::Output {
        DatexExpression {
            data: self,
            span,
            wrapped: None,
        }
    }

    fn with_default_span(self) -> Self::Output {
        DatexExpression {
            data: self,
            span: SimpleSpan::from(0..0),
            wrapped: None,
        }
    }
}

impl Spanned for TypeExpressionData {
    type Output = TypeExpression;

    fn with_span(self, span: SimpleSpan) -> Self::Output {
        TypeExpression {
            data: self,
            span,
            wrapped: None,
        }
    }

    fn with_default_span(self) -> Self::Output {
        TypeExpression {
            data: self,
            span: SimpleSpan::from(0..0),
            wrapped: None,
        }
    }
}

// directly convert DatexExpression to a ValueContainer
impl TryFrom<&DatexExpressionData> for ValueContainer {
    type Error = ();

    fn try_from(expr: &DatexExpressionData) -> Result<Self, Self::Error> {
        Ok(match expr {
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator,
                expression,
            }) => {
                let value = ValueContainer::try_from(&expression.data)?;
                match value {
                    ValueContainer::Value(Value {
                        inner: CoreValue::Integer(_) | CoreValue::Decimal(_),
                        ..
                    }) => match operator {
                        UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Plus,
                        ) => value,
                        UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Minus,
                        ) => value.neg().map_err(|_| ())?,
                        _ => Err(())?,
                    },
                    _ => Err(())?,
                }
            }
            DatexExpressionData::Null => ValueContainer::Value(Value::null()),
            DatexExpressionData::Boolean(b) => ValueContainer::from(*b),
            DatexExpressionData::Text(s) => ValueContainer::from(s.clone()),
            DatexExpressionData::Decimal(d) => ValueContainer::from(d.clone()),
            DatexExpressionData::Integer(i) => ValueContainer::from(i.clone()),
            DatexExpressionData::Endpoint(e) => ValueContainer::from(e.clone()),
            DatexExpressionData::List(list) => {
                let entries = list
                    .items
                    .iter()
                    .map(|e| ValueContainer::try_from(&e.data))
                    .collect::<Result<Vec<ValueContainer>, ()>>()?;
                ValueContainer::from(
                    datex_core::values::core_values::list::List::from(entries),
                )
            }
            DatexExpressionData::Map(pairs) => {
                let entries = pairs
                    .entries
                    .iter()
                    .map(|(k, v)| {
                        let key = ValueContainer::try_from(&k.data)?;
                        let value = ValueContainer::try_from(&v.data)?;
                        Ok((key, value))
                    })
                    .collect::<Result<Vec<(ValueContainer, ValueContainer)>, ()>>()?;
                ValueContainer::from(
                    crate::values::core_values::map::Map::from(entries),
                )
            }
            _ => Err(())?,
        })
    }
}
