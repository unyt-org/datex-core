use std::ops::Range;

use crate::ast::chain::ApplyOperation;
use crate::ast::data::expression::{
    ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
    DatexExpression, DatexExpressionData, DerefAssignment, FunctionDeclaration,
    List, Map, RemoteExecution, Slot, SlotAssignment, Statements,
    TypeDeclaration, UnaryOperation, VariableAccess, VariableAssignment,
    VariableDeclaration,
};
use crate::ast::data::visitor::Visit;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;

#[derive(Debug, Clone)]
/// Actions that can be taken when visiting an expression
pub enum VisitAction {
    /// Continue visiting child nodes
    VisitChildren,
    /// Skip visiting child nodes
    SkipChildren,
    /// Replace the current node with a new one, skipping child nodes
    Replace(DatexExpression),
    /// Recurse into child nodes, then replace the current node with a new one
    ReplaceRecurseChildNodes(DatexExpression),
    /// Replace the current node with a new one, and recurse into it
    ReplaceRecurse(DatexExpression),
    /// Convert the current node to a no-op
    ToNoop,
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
impl VisitableExpression for List {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        for item in &mut self.items {
            visitor.visit_datex_expression(item);
        }
    }
}
impl VisitableExpression for Map {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        for (_key, value) in &mut self.entries {
            visitor.visit_datex_expression(value);
        }
    }
}
impl VisitableExpression for Conditional {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.condition);
        visitor.visit_datex_expression(&mut self.then_branch);
        if let Some(else_branch) = &mut self.else_branch {
            visitor.visit_datex_expression(else_branch);
        }
    }
}
impl VisitableExpression for VariableDeclaration {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.init_expression);
    }
}
impl VisitableExpression for VariableAssignment {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.expression);
    }
}
impl VisitableExpression for UnaryOperation {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.expression);
    }
}
impl VisitableExpression for TypeDeclaration {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        todo!()
    }
}
impl VisitableExpression for ComparisonOperation {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.left);
        visitor.visit_datex_expression(&mut self.right);
    }
}
impl VisitableExpression for DerefAssignment {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.assigned_expression);
        visitor.visit_datex_expression(&mut self.deref_expression);
    }
}
impl VisitableExpression for ApplyChain {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.base);
        for operation in &mut self.operations {
            match operation {
                ApplyOperation::FunctionCall(arg) => {
                    visitor.visit_datex_expression(arg);
                }
                ApplyOperation::GenericAccess(arg) => {
                    visitor.visit_datex_expression(arg);
                }
                ApplyOperation::PropertyAccess(prop) => {
                    visitor.visit_datex_expression(prop);
                }
            }
        }
    }
}
impl VisitableExpression for RemoteExecution {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.left);
        visitor.visit_datex_expression(&mut self.right);
    }
}
impl VisitableExpression for SlotAssignment {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        visitor.visit_datex_expression(&mut self.expression);
    }
}
impl VisitableExpression for FunctionDeclaration {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        for (_, param_type) in &mut self.parameters {
            // FIXME //visitor.visit_type_expression(param_type);
        }
        visitor.visit_datex_expression(&mut self.body);
    }
}

impl VisitableExpression for DatexExpression {
    fn walk_children(&mut self, visitor: &mut impl ExpressionVisitor) {
        match &mut self.data {
            DatexExpressionData::BinaryOperation(op) => {
                op.walk_children(visitor)
            }
            DatexExpressionData::Statements(statements) => {
                statements.walk_children(visitor)
            }
            DatexExpressionData::List(list) => list.walk_children(visitor),
            DatexExpressionData::Map(map) => map.walk_children(visitor),
            DatexExpressionData::Conditional(conditional) => {
                conditional.walk_children(visitor)
            }
            DatexExpressionData::VariableDeclaration(variable_declaration) => {
                variable_declaration.walk_children(visitor)
            }
            DatexExpressionData::VariableAssignment(variable_assignment) => {
                variable_assignment.walk_children(visitor)
            }
            DatexExpressionData::TypeDeclaration(type_declaration) => {
                type_declaration.walk_children(visitor)
            }
            DatexExpressionData::TypeExpression(type_expression) => {
                //type_expression.walk_children(visitor)
            }
            DatexExpressionData::Type(type_expression) => {
                // type_expression.walk_children(visitor)
            }
            DatexExpressionData::FunctionDeclaration(function_declaration) => {
                function_declaration.walk_children(visitor)
            }
            DatexExpressionData::CreateRef(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::CreateRefMut(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::CreateRefFinal(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::Deref(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::SlotAssignment(slot_assignment) => {
                slot_assignment.walk_children(visitor)
            }
            DatexExpressionData::ComparisonOperation(comparison_operation) => {
                comparison_operation.walk_children(visitor)
            }
            DatexExpressionData::DerefAssignment(deref_assignment) => {
                deref_assignment.walk_children(visitor)
            }
            DatexExpressionData::UnaryOperation(unary_operation) => {
                unary_operation.walk_children(visitor)
            }
            DatexExpressionData::ApplyChain(apply_chain) => {
                apply_chain.walk_children(visitor)
            }
            DatexExpressionData::RemoteExecution(remote_execution) => {
                remote_execution.walk_children(visitor)
            }

            DatexExpressionData::Noop
            | DatexExpressionData::PointerAddress(_)
            | DatexExpressionData::VariableAccess(_)
            | DatexExpressionData::GetReference(_)
            | DatexExpressionData::Slot(_)
            | DatexExpressionData::Placeholder
            | DatexExpressionData::Recover
            | DatexExpressionData::Null
            | DatexExpressionData::Boolean(_)
            | DatexExpressionData::Text(_)
            | DatexExpressionData::Decimal(_)
            | DatexExpressionData::TypedDecimal(_)
            | DatexExpressionData::Integer(_)
            | DatexExpressionData::TypedInteger(_)
            | DatexExpressionData::Identifier(_)
            | DatexExpressionData::Endpoint(_) => {}
        }
    }
}

pub trait ExpressionVisitor: Sized {
    fn visit_datex_expression(&mut self, expr: &mut DatexExpression) {
        let action = match &mut expr.data {
            DatexExpressionData::UnaryOperation(op) => {
                self.visit_unary_operation(op, &expr.span)
            }
            DatexExpressionData::Statements(stmts) => {
                self.visit_statements(stmts, &expr.span)
            }
            DatexExpressionData::VariableDeclaration(var_decl) => {
                self.visit_variable_declaration(var_decl, &expr.span)
            }
            DatexExpressionData::VariableAssignment(var_assign) => {
                self.visit_variable_assignment(var_assign, &expr.span)
            }
            DatexExpressionData::VariableAccess(var_access) => {
                self.visit_variable_access(var_access, &expr.span)
            }
            DatexExpressionData::Integer(i) => {
                self.visit_integer(i, &expr.span)
            }
            DatexExpressionData::TypedInteger(ti) => {
                self.visit_typed_integer(ti, &expr.span)
            }
            DatexExpressionData::Decimal(d) => {
                self.visit_decimal(d, &expr.span)
            }
            DatexExpressionData::TypedDecimal(td) => {
                self.visit_typed_decimal(td, &expr.span)
            }
            DatexExpressionData::Text(s) => self.visit_text(s, &expr.span),
            DatexExpressionData::Boolean(b) => {
                self.visit_boolean(b, &expr.span)
            }
            DatexExpressionData::Endpoint(e) => {
                self.visit_endpoint(e, &expr.span)
            }
            DatexExpressionData::Null => self.visit_null(&expr.span),
            DatexExpressionData::List(list) => {
                self.visit_list(list, &expr.span)
            }
            DatexExpressionData::Map(map) => self.visit_map(map, &expr.span),
            DatexExpressionData::GetReference(pointer_address) => {
                self.visit_get_reference(pointer_address, &expr.span)
            }
            DatexExpressionData::Conditional(conditional) => {
                self.visit_conditional(conditional, &expr.span)
            }
            DatexExpressionData::TypeDeclaration(type_declaration) => {
                self.visit_type_declaration(type_declaration, &expr.span)
            }
            DatexExpressionData::TypeExpression(type_expression) => {
                unimplemented!("TypeExpression is going to be deprecated");
                //self.visit_type_expression(type_expression)
            }
            DatexExpressionData::Type(type_expression) => {
                unimplemented!("TypeExpression is going to be deprecated");
                //self.visit_type_expression(type_expression)
            }
            DatexExpressionData::FunctionDeclaration(function_declaration) => {
                self.visit_function_declaration(
                    function_declaration,
                    &expr.span,
                )
            }
            DatexExpressionData::CreateRef(datex_expression) => {
                self.visit_create_ref(datex_expression, &expr.span)
            }
            DatexExpressionData::CreateRefMut(datex_expression) => {
                self.visit_create_mut(datex_expression, &expr.span)
            }
            DatexExpressionData::Deref(deref) => {
                self.visit_deref(deref, &expr.span)
            }
            DatexExpressionData::Slot(slot) => {
                self.visit_slot(slot, &expr.span)
            }
            DatexExpressionData::SlotAssignment(slot_assignment) => {
                self.visit_slot_assignment(slot_assignment, &expr.span)
            }
            DatexExpressionData::PointerAddress(pointer_address) => {
                self.visit_pointer_address(pointer_address, &expr.span)
            }
            DatexExpressionData::BinaryOperation(binary_operation) => {
                self.visit_binary_operation(binary_operation, &expr.span)
            }
            DatexExpressionData::ComparisonOperation(comparison_operation) => {
                self.visit_comparison_operation(
                    comparison_operation,
                    &expr.span,
                )
            }
            DatexExpressionData::DerefAssignment(deref_assignment) => {
                self.visit_deref_assignment(deref_assignment, &expr.span)
            }
            DatexExpressionData::ApplyChain(apply_chain) => {
                self.visit_apply_chain(apply_chain, &expr.span)
            }
            DatexExpressionData::RemoteExecution(remote_execution) => {
                self.visit_remote_execution(remote_execution, &expr.span)
            }
            DatexExpressionData::CreateRefFinal(datex_expression) => {
                unimplemented!("CreateRefFinal is going to be deprecated")
            }
            DatexExpressionData::Identifier(identifier) => {
                self.visit_identifier(identifier, &expr.span)
            }
            DatexExpressionData::Placeholder | DatexExpressionData::Recover => {
                unreachable!(
                    "Placeholder and Recover expressions should not be visited"
                )
            }
            DatexExpressionData::Noop => VisitAction::SkipChildren,
        };

        match action {
            VisitAction::SkipChildren => {}
            VisitAction::ToNoop => {
                expr.data = DatexExpressionData::Noop;
            }
            VisitAction::VisitChildren => expr.walk_children(self),
            VisitAction::Replace(new_expr) => *expr = new_expr,
            VisitAction::ReplaceRecurseChildNodes(new_expr) => {
                expr.walk_children(self);
                *expr = new_expr;
            }
            VisitAction::ReplaceRecurse(new_expr) => {
                *expr = new_expr;
                self.visit_datex_expression(expr);
            }
        }
    }

    /// Visit statements
    fn visit_statements(
        &mut self,
        stmts: &mut Statements,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit unary operation
    fn visit_unary_operation(
        &mut self,
        op: &mut UnaryOperation,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit conditional expression
    fn visit_conditional(
        &mut self,
        cond: &mut Conditional,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit type declaration
    fn visit_type_declaration(
        &mut self,
        type_decl: &mut TypeDeclaration,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit binary operation
    fn visit_binary_operation(
        &mut self,
        op: &mut BinaryOperation,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit comparison operation
    fn visit_comparison_operation(
        &mut self,
        op: &mut ComparisonOperation,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit dereference assignment
    fn visit_deref_assignment(
        &mut self,
        deref_assign: &mut DerefAssignment,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit apply chain
    fn visit_apply_chain(
        &mut self,
        apply_chain: &mut ApplyChain,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit remote execution
    fn visit_remote_execution(
        &mut self,
        remote_execution: &mut RemoteExecution,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit function declaration
    fn visit_function_declaration(
        &mut self,
        func_decl: &mut FunctionDeclaration,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit slot assignment
    fn visit_slot_assignment(
        &mut self,
        slot_assign: &mut SlotAssignment,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit variable declaration
    fn visit_variable_declaration(
        &mut self,
        var_decl: &mut VariableDeclaration,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit variable assignment
    fn visit_variable_assignment(
        &mut self,
        var_assign: &mut VariableAssignment,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit variable access
    fn visit_variable_access(
        &mut self,
        _var_access: &mut VariableAccess,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit create reference expression
    fn visit_create_ref(
        &mut self,
        datex_expression: &mut DatexExpression,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit create mutable reference expression
    fn visit_create_mut(
        &mut self,
        _datex_expression: &mut DatexExpression,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit dereference expression
    fn visit_deref(
        &mut self,
        _datex_expression: &mut DatexExpression,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit list expression
    fn visit_list(
        &mut self,
        _list: &mut List,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit map expression
    fn visit_map(
        &mut self,
        _map: &mut Map,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }

    /// Visit integer literal
    fn visit_integer(
        &mut self,
        _integer: &mut Integer,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit typed integer literal
    fn visit_typed_integer(
        &mut self,
        _typed_integer: &TypedInteger,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit decimal literal
    fn visit_decimal(
        &mut self,
        _decimal: &mut Decimal,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit typed decimal literal
    fn visit_typed_decimal(
        &mut self,
        _typed_decimal: &TypedDecimal,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit identifier
    fn visit_identifier(
        &mut self,
        _identifier: &mut String,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit text literal
    fn visit_text(
        &mut self,
        _text: &mut String,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit get reference expression
    fn visit_get_reference(
        &mut self,
        _pointer_address: &mut PointerAddress,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit boolean literal
    fn visit_boolean(
        &mut self,
        _boolean: &mut bool,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit endpoint expression
    fn visit_endpoint(
        &mut self,
        _endpoint: &mut Endpoint,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit null literal
    fn visit_null(&mut self, _span: &Range<usize>) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit pointer address expression
    fn visit_pointer_address(
        &mut self,
        _pointer_address: &PointerAddress,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }

    /// Visit slot expression
    fn visit_slot(
        &mut self,
        _slot: &Slot,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::SkipChildren
    }
}

struct MyAst;
impl ExpressionVisitor for MyAst {
    fn visit_identifier(
        &mut self,
        identifier: &mut String,
        span: &Range<usize>,
    ) -> VisitAction {
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
        _span: &Range<usize>,
    ) -> VisitAction {
        println!("visit create ref {:?}", datex_expression);
        VisitAction::VisitChildren
    }
    fn visit_statements(
        &mut self,
        _statements: &mut Statements,
        _span: &Range<usize>,
    ) -> VisitAction {
        VisitAction::VisitChildren
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{binary_operation::BinaryOperator, parse};

    use super::*;

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
