pub mod visitable;
use std::ops::Range;

use crate::ast::structs::expression::{
    ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
    DatexExpression, DatexExpressionData, DerefAssignment, FunctionDeclaration,
    List, Map, RemoteExecution, Slot, SlotAssignment, Statements,
    TypeDeclaration, UnaryOperation, VariableAccess, VariableAssignment,
    VariableDeclaration,
};
use crate::ast::structs::r#type::TypeExpression;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;
use crate::visitor::expression::visitable::{
    ExpressionVisitAction, VisitableExpression,
};
use crate::visitor::type_expression::TypeExpressionVisitor;
use crate::visitor::{ErrorWithVisitAction, VisitAction};

pub struct EmptyExpressionError;
impl ErrorWithVisitAction<DatexExpression> for EmptyExpressionError {
    fn with_visit_action(self, _action: &VisitAction<DatexExpression>) {}
    fn visit_action(&self) -> &VisitAction<DatexExpression> {
        &VisitAction::SkipChildren
    }
}
pub type EmptyExpressionVisitAction =
    ExpressionVisitAction<EmptyExpressionError>;

pub trait ExpressionVisitor<
    T: ErrorWithVisitAction<DatexExpression>,
    X: ErrorWithVisitAction<TypeExpression>,
>: TypeExpressionVisitor<X>
{
    fn visit_datex_expression(&mut self, expr: &mut DatexExpression) {
        let visit_result = match &mut expr.data {
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
                self.visit_type_expression(type_expression);
                Ok(VisitAction::SkipChildren)
            }
            DatexExpressionData::Type(type_expression) => {
                self.visit_type_expression(type_expression);
                Ok(VisitAction::SkipChildren)
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
            DatexExpressionData::Noop => Ok(VisitAction::SkipChildren),
        };

        let action = match &visit_result {
            Ok(act) => act,
            Err(error) => error.visit_action(),
        };
        match action {
            VisitAction::SkipChildren => {}
            VisitAction::ToNoop => {
                expr.data = DatexExpressionData::Noop;
            }
            VisitAction::VisitChildren => expr.walk_children(self),
            VisitAction::Replace(new_expr) => *expr = new_expr.to_owned(),
            VisitAction::ReplaceRecurseChildNodes(new_expr) => {
                expr.walk_children(self);
                *expr = new_expr.to_owned();
            }
            VisitAction::ReplaceRecurse(new_expr) => {
                *expr = new_expr.to_owned();
                self.visit_datex_expression(expr);
            }
        }
    }

    /// Visit statements
    fn visit_statements(
        &mut self,
        statements: &mut Statements,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = statements;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit unary operation
    fn visit_unary_operation(
        &mut self,
        unary_operation: &mut UnaryOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = unary_operation;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit conditional expression
    fn visit_conditional(
        &mut self,
        conditional: &mut Conditional,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = conditional;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit type declaration
    fn visit_type_declaration(
        &mut self,
        type_declaration: &mut TypeDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = type_declaration;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit binary operation
    fn visit_binary_operation(
        &mut self,
        binary_operation: &mut BinaryOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = binary_operation;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit comparison operation
    fn visit_comparison_operation(
        &mut self,
        comparison_operation: &mut ComparisonOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = comparison_operation;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit dereference assignment
    fn visit_deref_assignment(
        &mut self,
        deref_assignment: &mut DerefAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = deref_assignment;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit apply chain
    fn visit_apply_chain(
        &mut self,
        apply_chain: &mut ApplyChain,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = apply_chain;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit remote execution
    fn visit_remote_execution(
        &mut self,
        remote_execution: &mut RemoteExecution,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = remote_execution;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit function declaration
    fn visit_function_declaration(
        &mut self,
        function_declaration: &mut FunctionDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = function_declaration;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit slot assignment
    fn visit_slot_assignment(
        &mut self,
        slot_assignment: &mut SlotAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = slot_assignment;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit variable declaration
    fn visit_variable_declaration(
        &mut self,
        variable_declaration: &mut VariableDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = variable_declaration;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit variable assignment
    fn visit_variable_assignment(
        &mut self,
        variable_assignment: &mut VariableAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = variable_assignment;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit variable access
    fn visit_variable_access(
        &mut self,
        var_access: &mut VariableAccess,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = var_access;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit create reference expression
    fn visit_create_ref(
        &mut self,
        datex_expression: &mut DatexExpression,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = datex_expression;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit create mutable reference expression
    fn visit_create_mut(
        &mut self,
        datex_expression: &mut DatexExpression,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = datex_expression;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit dereference expression
    fn visit_deref(
        &mut self,
        datex_expression: &mut DatexExpression,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = datex_expression;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit list expression
    fn visit_list(
        &mut self,
        list: &mut List,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = list;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit map expression
    fn visit_map(
        &mut self,
        map: &mut Map,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = map;
        let _ = span;
        Ok(VisitAction::VisitChildren)
    }

    /// Visit integer literal
    fn visit_integer(
        &mut self,
        integer: &mut Integer,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = integer;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit typed integer literal
    fn visit_typed_integer(
        &mut self,
        typed_integer: &TypedInteger,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = typed_integer;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit decimal literal
    fn visit_decimal(
        &mut self,
        decimal: &mut Decimal,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = decimal;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit typed decimal literal
    fn visit_typed_decimal(
        &mut self,
        typed_decimal: &TypedDecimal,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = typed_decimal;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit identifier
    fn visit_identifier(
        &mut self,
        identifier: &mut String,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = identifier;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit text literal
    fn visit_text(
        &mut self,
        text: &mut String,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = text;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit get reference expression
    fn visit_get_reference(
        &mut self,
        pointer_address: &mut PointerAddress,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = pointer_address;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit boolean literal
    fn visit_boolean(
        &mut self,
        boolean: &mut bool,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = boolean;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit endpoint expression
    fn visit_endpoint(
        &mut self,
        endpoint: &mut Endpoint,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = endpoint;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit null literal
    fn visit_null(&mut self, span: &Range<usize>) -> ExpressionVisitAction<T> {
        let _ = span;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit pointer address expression
    fn visit_pointer_address(
        &mut self,
        pointer_address: &PointerAddress,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = pointer_address;
        Ok(VisitAction::SkipChildren)
    }

    /// Visit slot expression
    fn visit_slot(
        &mut self,
        slot: &Slot,
        span: &Range<usize>,
    ) -> ExpressionVisitAction<T> {
        let _ = span;
        let _ = slot;
        Ok(VisitAction::SkipChildren)
    }
}
