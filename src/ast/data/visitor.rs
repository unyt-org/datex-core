use chumsky::span::SimpleSpan;

use crate::{
    ast::data::{
        expression::{
            ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
            DatexExpression, DerefAssignment, FunctionDeclaration, List, Map,
            RemoteExecution, Slot, SlotAssignment, Statements, TypeDeclaration,
            UnaryOperation, VariableAccess, VariableAssignment,
            VariableDeclaration,
        },
        r#type::{
            FixedSizeList, FunctionType, GenericAccess, Intersection,
            SliceList, StructuralList, StructuralMap, TypeExpression, Union,
        },
    },
    values::core_values::{
        decimal::{Decimal, typed_decimal::TypedDecimal},
        endpoint::Endpoint,
        integer::{Integer, typed_integer::TypedInteger},
    },
};

use crate::values::pointer::PointerAddress;
pub trait Visitable {
    fn visit_children_with(&self, visitor: &mut impl Visit);
}

/// Visitor pattern for traversing the AST
/// Implement the `Visit` trait and override the methods for the nodes you want to visit.
/// The default implementation visits all child nodes and traverses the entire tree.
pub trait Visit: Sized {
    // Type Expressions
    fn visit_type_expression(&mut self, type_expr: &TypeExpression) {
        type_expr.visit_children_with(self);
    }
    fn visit_structural_list(
        &mut self,
        structural_list: &StructuralList,
        _span: SimpleSpan,
    ) {
        structural_list.visit_children_with(self);
    }
    fn visit_fixed_size_list(
        &mut self,
        fixed_size_list: &FixedSizeList,
        _span: SimpleSpan,
    ) {
        fixed_size_list.visit_children_with(self);
    }
    fn visit_slice_list(&mut self, slice_list: &SliceList, _span: SimpleSpan) {
        slice_list.visit_children_with(self);
    }
    fn visit_intersection(
        &mut self,
        intersection: &Intersection,
        _span: SimpleSpan,
    ) {
        intersection.visit_children_with(self);
    }
    fn visit_union(&mut self, union: &Union, _span: SimpleSpan) {
        union.visit_children_with(self);
    }
    fn visit_generic_access(
        &mut self,
        generic_access: &GenericAccess,
        _span: SimpleSpan,
    ) {
        generic_access.visit_children_with(self);
    }
    fn visit_function_type(
        &mut self,
        function_type: &FunctionType,
        _span: SimpleSpan,
    ) {
        function_type.visit_children_with(self);
    }
    fn visit_structural_map(
        &mut self,
        structural_map: &StructuralMap,
        _span: SimpleSpan,
    ) {
        structural_map.visit_children_with(self);
    }
    fn visit_type_ref(&mut self, type_ref: &TypeExpression, _span: SimpleSpan) {
        type_ref.visit_children_with(self);
    }
    fn visit_type_ref_mut(
        &mut self,
        type_ref_mut: &TypeExpression,
        _span: SimpleSpan,
    ) {
        type_ref_mut.visit_children_with(self);
    }

    // Expressions
    fn visit_expression(&mut self, expr: &DatexExpression) {
        expr.visit_children_with(self);
    }
    fn visit_statements(&mut self, stmts: &Statements, _span: SimpleSpan) {
        stmts.visit_children_with(self);
    }
    fn visit_unary_operation(
        &mut self,
        op: &UnaryOperation,
        _span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }
    fn visit_conditional(&mut self, cond: &Conditional, _span: SimpleSpan) {
        cond.visit_children_with(self);
    }
    fn visit_type_declaration(
        &mut self,
        type_decl: &TypeDeclaration,
        _span: SimpleSpan,
    ) {
        type_decl.visit_children_with(self);
    }
    fn visit_binary_operation(
        &mut self,
        op: &BinaryOperation,
        _span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }
    fn visit_comparison_operation(
        &mut self,
        op: &ComparisonOperation,
        _span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }
    fn visit_deref_assignment(
        &mut self,
        deref_assign: &DerefAssignment,
        _span: SimpleSpan,
    ) {
        deref_assign.visit_children_with(self);
    }
    fn visit_apply_chain(
        &mut self,
        apply_chain: &ApplyChain,
        _span: SimpleSpan,
    ) {
        apply_chain.visit_children_with(self);
    }
    fn visit_remote_execution(
        &mut self,
        remote_execution: &RemoteExecution,
        _span: SimpleSpan,
    ) {
        remote_execution.visit_children_with(self);
    }
    fn visit_function_declaration(
        &mut self,
        func_decl: &FunctionDeclaration,
        _span: SimpleSpan,
    ) {
        func_decl.visit_children_with(self);
    }
    fn visit_slot_assignment(
        &mut self,
        slot_assign: &SlotAssignment,
        _span: SimpleSpan,
    ) {
        slot_assign.visit_children_with(self);
    }
    fn visit_variable_declaration(
        &mut self,
        var_decl: &VariableDeclaration,
        _span: SimpleSpan,
    ) {
        var_decl.visit_children_with(self);
    }
    fn visit_variable_assignment(
        &mut self,
        var_assign: &VariableAssignment,
        _span: SimpleSpan,
    ) {
        var_assign.visit_children_with(self);
    }
    fn visit_variable_access(
        &mut self,
        _var_access: &VariableAccess,
        _span: SimpleSpan,
    ) {
    }
    fn visit_create_ref(
        &mut self,
        datex_expression: &DatexExpression,
        _span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }
    fn visit_create_mut(
        &mut self,
        datex_expression: &DatexExpression,
        _span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }
    fn visit_deref(
        &mut self,
        datex_expression: &DatexExpression,
        _span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }
    fn visit_list(&mut self, list: &List, _span: SimpleSpan) {
        list.visit_children_with(self);
    }
    fn visit_map(&mut self, map: &Map, _span: SimpleSpan) {
        map.visit_children_with(self);
    }
    fn visit_integer(&mut self, _value: &Integer, _span: SimpleSpan) {}
    fn visit_typed_integer(
        &mut self,
        _value: &TypedInteger,
        _span: SimpleSpan,
    ) {
    }
    fn visit_decimal(&mut self, _value: &Decimal, _span: SimpleSpan) {}
    fn visit_typed_decimal(
        &mut self,
        _value: &TypedDecimal,
        _span: SimpleSpan,
    ) {
    }
    fn visit_text(&mut self, _value: &String, _span: SimpleSpan) {}
    fn visit_get_reference(
        &mut self,
        _pointer_address: &PointerAddress,
        _span: SimpleSpan,
    ) {
    }
    fn visit_boolean(&mut self, _value: bool, _span: SimpleSpan) {}
    fn visit_endpoint(&mut self, _value: &Endpoint, _span: SimpleSpan) {}
    fn visit_null(&mut self, _span: SimpleSpan) {}
    fn visit_pointer_address(
        &mut self,
        _pointer_address: &PointerAddress,
        _span: SimpleSpan,
    ) {
    }
    fn visit_slot(&mut self, _slot: &Slot, _span: SimpleSpan) {}
}
