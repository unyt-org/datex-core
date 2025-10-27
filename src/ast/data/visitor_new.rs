use std::ops::Range;

use crate::{
    ast::data::{
        expression::{
            ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
            DatexExpression, DatexExpressionData, DerefAssignment,
            FunctionDeclaration, List, Map, RemoteExecution, Slot,
            SlotAssignment, Statements, TypeDeclaration, UnaryOperation,
            VariableAccess, VariableAssignment, VariableDeclaration,
        },
        r#type::{
            FixedSizeList, FunctionType, GenericAccess, Intersection,
            SliceList, StructuralList, StructuralMap, TypeExpression,
            TypeExpressionData, Union,
        },
    },
    values::core_values::{
        decimal::{Decimal, typed_decimal::TypedDecimal},
        endpoint::Endpoint,
        integer::{Integer, typed_integer::TypedInteger},
    },
};

pub enum VisitAction {
    VisitChildren,
    SkipChildren,
    Replace(DatexExpression),
    RecurseThenReplace(DatexExpression),
}
pub trait VisitableExpression {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor);
}

impl VisitableExpression for BinaryOperation {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.left);
        visitor.visit_datex_expression(&mut self.right);
    }
}

impl VisitableExpression for Statements {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        for item in &mut self.statements {
            visitor.visit_datex_expression(item);
        }
    }
}

impl VisitableExpression for DatexExpression {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        match &mut self.data {
            DatexExpressionData::BinaryOperation(op) => {
                op.walk_children(visitor)
            }
            DatexExpressionData::Statements(s) => s.walk_children(visitor),
            _ => {}
        }
    }
}

pub trait ExpressionVisitor: Sized {
    fn visit_datex_expression(&mut self, expr: &mut DatexExpression) {
        let action = match &mut expr.data {
            DatexExpressionData::Statements(s) => {
                self.visit_statements(s, &expr.span)
            }
            DatexExpressionData::Identifier(id) => {
                self.visit_identifier(id, &expr.span)
            }
            DatexExpressionData::BinaryOperation(op) => {
                self.visit_binary_operation(op, &expr.span)
            }
            DatexExpressionData::Boolean(_) => {
                self.visit_boolean(&mut expr.data, &expr.span)
            }
            _ => unreachable!(
                "Visitor method not implemented for this expression type"
            ),
        };

        match action {
            VisitAction::VisitChildren => expr.walk_children(self),
            VisitAction::SkipChildren => {}
            VisitAction::Replace(new_expr) => *expr = new_expr,
            VisitAction::RecurseThenReplace(new_expr) => {
                expr.walk_children(self);
                *expr = new_expr;
            }
        }
    }

    fn visit_statements(
        &mut self,
        _statements: &mut Statements,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }
    fn visit_identifier(
        &mut self,
        _identifier: &mut String,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }
    fn visit_literal(
        &mut self,
        _lit: &mut String,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }
    fn visit_binary_operation(
        &mut self,
        _op: &mut BinaryOperation,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }
    fn visit_boolean(
        &mut self,
        _data: &mut DatexExpressionData,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }
}

struct MyAst;
impl ExpressionVisitor for MyAst {
    fn visit_binary_operation(
        &mut self,
        _op: &mut BinaryOperation,
        span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::Replace(DatexExpression::new(
            DatexExpressionData::Boolean(true),
            span.clone(),
        ))
    }
    fn visit_statements(
        &mut self,
        _statements: &mut Statements,
        _span: &Range<usize>,
    ) -> VisitAction {
        println!("Visiting statements at span: {:?}", _span);
        VisitAction::VisitChildren
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::binary_operation::BinaryOperator;

    use super::*;

    #[test]
    fn test() {
        let mut ast = DatexExpression {
            data: DatexExpressionData::Statements(Statements {
                statements: vec![DatexExpression {
                    data: DatexExpressionData::BinaryOperation(
                        BinaryOperation {
                            operator: BinaryOperator::VariantAccess,
                            left: Box::new(DatexExpression {
                                data: DatexExpressionData::Identifier(
                                    "x".to_string(),
                                ),
                                span: 0..1,
                                wrapped: None,
                            }),
                            right: Box::new(DatexExpression {
                                data: DatexExpressionData::Identifier(
                                    "y".to_string(),
                                ),
                                span: 2..3,
                                wrapped: None,
                            }),
                            r#type: None,
                        },
                    ),
                    wrapped: None,
                    span: 0..3,
                }],
                is_terminated: true,
            }),
            span: 1..2,
            wrapped: None,
        };
        let transformer = &mut MyAst;
        transformer.visit_datex_expression(&mut ast);
        println!("{:?}", ast);
    }
}
