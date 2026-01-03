use crate::values::core_values::r#type::Type;

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
    /// Set the type annotation of the current node, and recurse into child nodes
    SetTypeRecurseChildNodes(Type),
    /// Set the type annotation of the current node, skipping child nodes
    SetTypeSkipChildren(Type),
    /// Convert the current node to a no-op
    ToNoop,
}

#[cfg(test)]
mod tests {
    use crate::ast::expressions::CreateRef;
    use crate::ast::expressions::{
        BinaryOperation, DatexExpression, DatexExpressionData, Statements,
        VariableAccess,
    };
    use crate::ast::type_expressions::{TypeExpression, TypeExpressionData};
    use crate::global::operators::BinaryOperator;
    use crate::global::operators::binary::ArithmeticOperator;
    use crate::parser::Parser;
    use crate::visitor::{
        VisitAction, expression::visitable::ExpressionVisitResult,
    };
    use crate::visitor::{
        expression::ExpressionVisitor,
        type_expression::{
            TypeExpressionVisitor, visitable::TypeExpressionVisitResult,
        },
    };
    use core::ops::Range;

    pub struct MyAstTypeExpressionError {
        message: String,
    }

    #[derive(Debug)]
    pub struct MyAstExpressionError {
        message: String,
    }
    impl MyAstExpressionError {
        pub fn new(msg: &str) -> MyAstExpressionError {
            Self {
                message: msg.to_string(),
            }
        }
    }

    struct MyAst;
    impl TypeExpressionVisitor<MyAstExpressionError> for MyAst {
        fn visit_literal_type(
            &mut self,
            literal: &mut String,
            span: &Range<usize>,
        ) -> TypeExpressionVisitResult<MyAstExpressionError> {
            Ok(VisitAction::Replace(TypeExpression::new(
                TypeExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: "MYTYPE".to_string(),
                }),
                span.clone(),
            )))
        }
    }
    impl ExpressionVisitor<MyAstExpressionError> for MyAst {
        fn handle_expression_error(
            &mut self,
            error: MyAstExpressionError,
            expression: &DatexExpression,
        ) -> Result<VisitAction<DatexExpression>, MyAstExpressionError>
        {
            println!(
                "Expression error: {:?} at {:?}. Aborting...",
                error, expression.span
            );
            Err(error)
        }
        fn visit_create_ref(
            &mut self,
            create_ref: &mut CreateRef,
            span: &Range<usize>,
        ) -> ExpressionVisitResult<MyAstExpressionError> {
            Ok(VisitAction::VisitChildren)
        }

        fn visit_identifier(
            &mut self,
            identifier: &mut String,
            span: &Range<usize>,
        ) -> ExpressionVisitResult<MyAstExpressionError> {
            Ok(VisitAction::Replace(DatexExpression {
                data: DatexExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: identifier.clone(),
                }),
                span: span.clone(),
                wrapped: None,
                ty: None,
            }))
        }

        fn visit_boolean(
            &mut self,
            boolean: &mut bool,
            span: &Range<usize>,
        ) -> ExpressionVisitResult<MyAstExpressionError> {
            Err(MyAstExpressionError::new("Booleans are not allowed"))
        }
    }

    #[test]
    fn simple_test() {
        let mut ast =
            Parser::parse("var x: integer/u8 = 42; x; ((42 + x))").unwrap();
        MyAst.visit_datex_expression(&mut ast).unwrap();
        println!("{:#?}", ast);
    }

    #[test]
    fn error() {
        let mut ast = Parser::parse("true + false").unwrap();
        let mut transformer = MyAst;
        let res = transformer.visit_datex_expression(&mut ast);
        assert!(res.is_err());
    }

    #[test]
    fn test() {
        let mut ast = DatexExpression {
            data: DatexExpressionData::Statements(Statements {
                statements: vec![DatexExpression {
                    data: DatexExpressionData::BinaryOperation(
                        BinaryOperation {
                            operator: BinaryOperator::Arithmetic(
                                ArithmeticOperator::Add,
                            ),
                            left: Box::new(DatexExpression {
                                data: DatexExpressionData::Identifier(
                                    "x".to_string(),
                                ),
                                span: 0..1,
                                wrapped: None,
                                ty: None,
                            }),
                            right: Box::new(DatexExpression {
                                data: DatexExpressionData::Identifier(
                                    "y".to_string(),
                                ),
                                span: 2..3,
                                wrapped: None,
                                ty: None,
                            }),
                            ty: None,
                        },
                    ),
                    wrapped: None,
                    span: 0..3,
                    ty: None,
                }],
                is_terminated: true,
                unbounded: None,
            }),
            span: 1..2,
            wrapped: None,
            ty: None,
        };
        let transformer = &mut MyAst;
        transformer.visit_datex_expression(&mut ast).unwrap();
        println!("{:?}", ast);
    }
}
