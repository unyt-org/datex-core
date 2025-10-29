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

pub trait ErrorWithVisitAction<T: Sized> {
    fn with_visit_action(&mut self, action: VisitAction<T>);
    fn visit_action(&self) -> &VisitAction<T>;
}

#[cfg(test)]
mod tests {
    use crate::visitor::type_expression::EmptyTypeExpressionError;
    use crate::visitor::{
        VisitAction, expression::visitable::ExpressionVisitAction,
    };
    use crate::{
        ast::{
            parse,
            structs::{
                expression::{
                    BinaryOperation, DatexExpression, DatexExpressionData,
                    Statements,
                },
                operator::BinaryOperator,
            },
        },
        visitor::ErrorWithVisitAction,
    };
    use std::ops::Range;

    use crate::ast::structs::{
        expression::VariableAccess,
        r#type::{TypeExpression, TypeExpressionData},
    };
    use crate::visitor::{
        expression::ExpressionVisitor,
        type_expression::{
            TypeExpressionVisitor, visitable::TypeExpressionVisitAction,
        },
    };

    pub struct MyAstTypeExpressionError {
        message: String,
        action: VisitAction<TypeExpression>,
    }
    impl ErrorWithVisitAction<TypeExpression> for MyAstTypeExpressionError {
        fn visit_action(&self) -> &VisitAction<TypeExpression> {
            &self.action
        }
        fn with_visit_action(&mut self, action: VisitAction<TypeExpression>) {
            self.action = action;
        }
    }

    #[derive(Debug)]
    pub struct MyAstExpressionError {
        message: String,
        action: VisitAction<DatexExpression>,
    }
    impl MyAstExpressionError {
        pub fn new(msg: &str) -> MyAstExpressionError {
            Self {
                message: msg.to_string(),
                action: VisitAction::SkipChildren,
            }
        }
    }
    impl ErrorWithVisitAction<DatexExpression> for MyAstExpressionError {
        fn visit_action(&self) -> &VisitAction<DatexExpression> {
            &self.action
        }
        fn with_visit_action(&mut self, action: VisitAction<DatexExpression>) {
            self.action = action;
        }
    }

    struct MyAst;
    impl TypeExpressionVisitor<EmptyTypeExpressionError> for MyAst {
        fn visit_literal_type(
            &mut self,
            literal: &mut String,
            span: &Range<usize>,
        ) -> TypeExpressionVisitAction<EmptyTypeExpressionError> {
            Ok(VisitAction::Replace(TypeExpression::new(
                TypeExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: "MYTYPE".to_string(),
                }),
                span.clone(),
            )))
        }
    }
    impl ExpressionVisitor<MyAstExpressionError, EmptyTypeExpressionError>
        for MyAst
    {
        fn visit_identifier(
            &mut self,
            identifier: &mut String,
            span: &Range<usize>,
        ) -> ExpressionVisitAction<MyAstExpressionError> {
            Ok(VisitAction::Replace(DatexExpression {
                data: DatexExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: identifier.clone(),
                }),
                span: span.clone(),
                wrapped: None,
            }))
        }
        fn visit_create_ref(
            &mut self,
            datex_expression: &mut DatexExpression,
            span: &Range<usize>,
        ) -> ExpressionVisitAction<MyAstExpressionError> {
            println!("visit create ref {:?}", datex_expression);
            Ok(VisitAction::VisitChildren)
        }

        fn visit_boolean(
            &mut self,
            boolean: &mut bool,
            span: &Range<usize>,
        ) -> ExpressionVisitAction<MyAstExpressionError> {
            Err(MyAstExpressionError::new("Booleans are not allowed"))
        }

        fn handle_expression_error<'a>(
            &mut self,
            error: &'a MyAstExpressionError,
            expr: &DatexExpression,
        ) -> Option<&'a VisitAction<DatexExpression>> {
            println!("Expression error: {:?} at {:?}", error, expr.span);
            None
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
    fn error() {
        let mut ast = parse("true + false").unwrap().ast;
        let mut transformer = MyAst;
        transformer.visit_datex_expression(&mut ast);
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
