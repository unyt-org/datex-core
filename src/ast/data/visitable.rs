use crate::ast::tree::{DatexExpression, TypeExpression};

pub trait Visitable {
    fn visit_children_with(&self, visitor: &mut impl Visit);
}

/// Visitor pattern for traversing the AST
/// Implement the `Visit` trait and override the methods for the nodes you want to visit.
/// The default implementation visits all child nodes and traverses the entire tree.
pub trait Visit: Sized {
    fn visit_expression(&mut self, expr: &DatexExpression) {
        expr.visit_children_with(self);
    }
    fn visit_type_expression(&mut self, type_expr: &TypeExpression) {
        type_expr.visit_children_with(self);
    }
    fn visit_statements(&mut self, stmts: &Statements, span: SimpleSpan) {
        stmts.visit_children_with(self);
    }
    fn visit_unary_operation(&mut self, op: &UnaryOperation, span: SimpleSpan) {
        op.visit_children_with(self);
    }
    fn visit_conditional(&mut self, cond: &Conditional, span: SimpleSpan) {
        cond.visit_children_with(self);
    }
    fn visit_type_declaration(
        &mut self,
        type_decl: &TypeDeclaration,
        span: SimpleSpan,
    ) {
        type_decl.visit_children_with(self);
    }
    fn visit_binary_operation(
        &mut self,
        op: &BinaryOperation,
        span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }
    fn visit_comparison_operation(
        &mut self,
        op: &ComparisonOperation,
        span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }
    fn visit_deref_assignment(
        &mut self,
        deref_assign: &DerefAssignment,
        span: SimpleSpan,
    ) {
        deref_assign.visit_children_with(self);
    }
    fn visit_apply_chain(
        &mut self,
        apply_chain: &ApplyChain,
        span: SimpleSpan,
    ) {
        apply_chain.visit_children_with(self);
    }
    fn visit_remote_execution(
        &mut self,
        remote_execution: &RemoteExecution,
        span: SimpleSpan,
    ) {
        remote_execution.visit_children_with(self);
    }
    fn visit_function_declaration(
        &mut self,
        func_decl: &FunctionDeclaration,
        span: SimpleSpan,
    ) {
        func_decl.visit_children_with(self);
    }
    fn visit_slot_assignment(
        &mut self,
        slot_assign: &SlotAssignment,
        span: SimpleSpan,
    ) {
        slot_assign.visit_children_with(self);
    }
    fn visit_variable_declaration(
        &mut self,
        var_decl: &VariableDeclaration,
        span: SimpleSpan,
    ) {
        var_decl.visit_children_with(self);
    }
    fn visit_variable_assignment(
        &mut self,
        var_assign: &VariableAssignment,
        span: SimpleSpan,
    ) {
        var_assign.visit_children_with(self);
    }
    fn visit_variable_access(
        &mut self,
        var_access: &VariableAccess,
        span: SimpleSpan,
    ) {
    }
    fn visit_create_ref(
        &mut self,
        datex_expression: &DatexExpression,
        span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }
    fn visit_create_mut(
        &mut self,
        datex_expression: &DatexExpression,
        span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }
    fn visit_deref(
        &mut self,
        datex_expression: &DatexExpression,
        span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }
    fn visit_list(&mut self, list: &List, span: SimpleSpan) {
        list.visit_children_with(self);
    }
    fn visit_map(&mut self, map: &Map, span: SimpleSpan) {
        map.visit_children_with(self);
    }
    fn visit_integer(&mut self, value: &Integer, span: SimpleSpan) {}
    fn visit_typed_integer(&mut self, value: &TypedInteger, span: SimpleSpan) {}
    fn visit_decimal(&mut self, value: &Decimal, span: SimpleSpan) {}
    fn visit_typed_decimal(&mut self, value: &TypedDecimal, span: SimpleSpan) {}
    fn visit_text(&mut self, value: &String, span: SimpleSpan) {}
    fn visit_get_reference(
        &mut self,
        pointer_address: &PointerAddress,
        span: SimpleSpan,
    ) {
    }
    fn visit_boolean(&mut self, value: bool, span: SimpleSpan) {}
    fn visit_endpoint(&mut self, value: &Endpoint, span: SimpleSpan) {}
    fn visit_null(&mut self, span: SimpleSpan) {}
    fn visit_pointer_address(
        &mut self,
        pointer_address: &PointerAddress,
        span: SimpleSpan,
    ) {
    }
    fn visit_slot(&mut self, slot: &Slot, span: SimpleSpan) {}
}
