

pub mod expression;
pub mod type_expression;

#[derive(Debug, Clone)]
/// Actions that can be taken when visiting an expression
pub enum VisitAction<T: Sized> {
    /// Continue visiting child nodes
    VisitChildren,
    /// Skip visiting child nodes
    SkipChildren,
    /// Replace the current node with a new one, skipping child nodes
    Replace(T),
    /// Recurse into child nodes, then replace the current node with a new one
    ReplaceRecurseChildNodes(T),
    /// Replace the current node with a new one, and recurse into it
    ReplaceRecurse(T),
    /// Convert the current node to a no-op
    ToNoop,
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        binary_operation::BinaryOperator,
        data::expression::{
            BinaryOperation, DatexExpression, DatexExpressionData, Statements,
        },
        parse,
        visitor::{VisitAction, expression::visitable::ExpressionVisitAction},
    };
    use std::ops::Range;

    use crate::ast::{
        data::{
            expression::VariableAccess,
            r#type::{TypeExpression, TypeExpressionData},
        },
        visitor::{
            expression::ExpressionVisitor,
            type_expression::{
                TypeExpressionVisitor, visitable::TypeExpressionVisitAction,
            },
        },
    };

    struct MyAst;
    impl TypeExpressionVisitor for MyAst {
        fn visit_literal_type(
            &mut self,
            literal: &mut String,
            span: &Range<usize>,
        ) -> TypeExpressionVisitAction {
            VisitAction::Replace(TypeExpression::new(
                TypeExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: "MYTYPE".to_string(),
                }),
                span.clone(),
            ))
        }
    }
    impl ExpressionVisitor for MyAst {
        fn visit_identifier(
            &mut self,
            identifier: &mut String,
            span: &Range<usize>,
        ) -> ExpressionVisitAction {
            VisitAction::Replace(DatexExpression {
                data: DatexExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: identifier.clone(),
                }),
                span: span.clone(),
                wrapped: None,
            })
        }
        fn visit_create_ref(
            &mut self,
            datex_expression: &mut DatexExpression,
            span: &Range<usize>,
        ) -> ExpressionVisitAction {
            println!("visit create ref {:?}", datex_expression);
            VisitAction::VisitChildren
        }
    }

    #[test]
    fn simple_test() {
        let mut ast =
            parse("var x: integer/u8 = 42; x; ((42 + x))").unwrap().ast;
        MyAst.visit_datex_expression(&mut ast);
        println!("{:#?}", ast);
    }

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
