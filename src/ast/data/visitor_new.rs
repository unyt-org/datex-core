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

pub trait VisitableExpression {
    fn visit_children_with(&mut self, visitor: &mut impl ExpressionVisitor);
}
pub trait TransformableExpression {
    fn transform_children_with(
        &mut self,
        transformer: &mut impl ExpressionTransformer,
    );
}

impl VisitableExpression for BinaryOperation {
    fn visit_children_with(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.left);
        visitor.visit_datex_expression(&mut self.right);
    }
}

pub trait ExpressionTransformer: Sized {
    fn transform_expression(
        &mut self,
        mut expr: DatexExpression,
    ) -> DatexExpression {
        expr.transform_children_with(self);
        expr
    }

    fn transform_binary_operation(
        &mut self,
        op: BinaryOperation,
    ) -> DatexExpressionData {
        DatexExpressionData::BinaryOperation(op)
    }
}

impl TransformableExpression for DatexExpression {
    fn transform_children_with(
        &mut self,
        transformer: &mut impl ExpressionTransformer,
    ) {
        self.data = match std::mem::take(&mut self.data) {
            DatexExpressionData::BinaryOperation(op) => {
                transformer.transform_binary_operation(op)
            }
            other => other,
        };
    }
}

impl VisitableExpression for DatexExpression {
    fn visit_children_with(&mut self, visitor: &mut impl ExpressionVisitor) {
        match &mut self.data {
            DatexExpressionData::Identifier(id) => {
                visitor.visit_identifier(id, &self.span)
            }
            DatexExpressionData::BinaryOperation(op) => {
                visitor.visit_binary_operation(op, &self.span);
            }
            _ => unreachable!(),
        }
    }
}

pub trait ExpressionVisitor: Sized {
    fn visit_datex_expression(&mut self, expr: &mut DatexExpression) {
        expr.visit_children_with(self);
    }
    fn visit_identifier(&mut self, _identifier: &String, _span: &Range<usize>) {
    }
    fn visit_literal(&mut self, _lit: &mut String, _span: &Range<usize>) {}
    fn visit_binary_operation(
        &mut self,
        op: &mut BinaryOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_with(self);
    }
}

pub trait TypeExpressionVisitor: Sized {
    fn visit_type_expression(&mut self, expr: &mut TypeExpression) {
        expr.visit_children_with(self);
    }
    fn visit_type_literal(&mut self, _lit: &mut String, _span: &Range<usize>) {}
    fn visit_fixed_size_list(
        &mut self,
        list: &mut FixedSizeList,
        _span: &Range<usize>,
    ) {
        list.visit_children_with(self);
    }
}

impl ExpressionTransformer for MyAst {
    fn transform_binary_operation(
        &mut self,
        op: BinaryOperation,
    ) -> DatexExpressionData {
        DatexExpressionData::Boolean(true)
    }
}
pub struct MyAst;
impl ExpressionVisitor for MyAst {
    fn visit_binary_operation(
        &mut self,
        op: &mut BinaryOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_with(self);
    }
}

// TypeExpression
pub trait VisitableTypeExpression {
    fn visit_children_with(&mut self, visitor: &mut impl TypeExpressionVisitor);
}
impl VisitableTypeExpression for FixedSizeList {
    fn visit_children_with(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor,
    ) {
        visitor.visit_type_expression(&mut self.r#type);
    }
}
impl VisitableTypeExpression for TypeExpression {
    fn visit_children_with(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor,
    ) {
        match &mut self.data {
            TypeExpressionData::Literal(_) => {}
            TypeExpressionData::FixedSizeList(f) => {
                f.visit_children_with(visitor)
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::binary_operation::BinaryOperator;

    use super::*;

    #[test]
    fn test() {
        let ast = DatexExpression {
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
        let mut transformer = MyAst;
        let transformed = transformer.transform_expression(ast);
        println!("{:?}", transformed);
    }
}
