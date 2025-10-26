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
    fn visit_children_with(&mut self, visitor: &mut impl Visit);
}

/// Visitor pattern for traversing the AST
/// Implement the `Visit` trait and override the methods for the nodes you want to visit.
/// The default implementation visits all child nodes and traverses the entire tree.
pub trait Visit: Sized {
    // Type Expressions
    fn visit_type_expression(&mut self, type_expr: &mut TypeExpression) {
        type_expr.visit_children_with(self);
    }

    /// Visit structural list type expression
    fn visit_structural_list(
        &mut self,
        structural_list: &mut StructuralList,
        _span: SimpleSpan,
    ) {
        structural_list.visit_children_with(self);
    }

    /// Visit fixed size list type expression
    fn visit_fixed_size_list(
        &mut self,
        fixed_size_list: &mut FixedSizeList,
        _span: SimpleSpan,
    ) {
        fixed_size_list.visit_children_with(self);
    }

    /// Visit slice list type expression
    fn visit_slice_list(
        &mut self,
        slice_list: &mut SliceList,
        _span: SimpleSpan,
    ) {
        slice_list.visit_children_with(self);
    }

    /// Visit intersection type expression
    fn visit_intersection(
        &mut self,
        intersection: &mut Intersection,
        _span: SimpleSpan,
    ) {
        intersection.visit_children_with(self);
    }

    /// Visit union type expression
    fn visit_union(&mut self, union: &mut Union, _span: SimpleSpan) {
        union.visit_children_with(self);
    }

    /// Visit generic access type expression
    fn visit_generic_access(
        &mut self,
        generic_access: &mut GenericAccess,
        _span: SimpleSpan,
    ) {
        generic_access.visit_children_with(self);
    }

    /// Visit function type expression
    fn visit_function_type(
        &mut self,
        function_type: &mut FunctionType,
        _span: SimpleSpan,
    ) {
        function_type.visit_children_with(self);
    }

    /// Visit structural map type expression
    fn visit_structural_map(
        &mut self,
        structural_map: &mut StructuralMap,
        _span: SimpleSpan,
    ) {
        structural_map.visit_children_with(self);
    }

    /// Visit type reference expression
    fn visit_type_ref(
        &mut self,
        type_ref: &mut TypeExpression,
        _span: SimpleSpan,
    ) {
        type_ref.visit_children_with(self);
    }

    /// Visit mutable type reference expression
    fn visit_type_ref_mut(
        &mut self,
        type_ref_mut: &mut TypeExpression,
        _span: SimpleSpan,
    ) {
        type_ref_mut.visit_children_with(self);
    }

    // Expressions

    /// Visit datex expression
    fn visit_expression(&mut self, expr: &mut DatexExpression) {
        expr.visit_children_with(self);
    }

    /// Visit statements
    fn visit_statements(&mut self, stmts: &mut Statements, _span: SimpleSpan) {
        stmts.visit_children_with(self);
    }

    /// Visit unary operation
    fn visit_unary_operation(
        &mut self,
        op: &mut UnaryOperation,
        _span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }

    /// Visit conditional expression
    fn visit_conditional(&mut self, cond: &mut Conditional, _span: SimpleSpan) {
        cond.visit_children_with(self);
    }

    /// Visit type declaration
    fn visit_type_declaration(
        &mut self,
        type_decl: &mut TypeDeclaration,
        _span: SimpleSpan,
    ) {
        type_decl.visit_children_with(self);
    }

    /// Visit binary operation
    fn visit_binary_operation(
        &mut self,
        op: &mut BinaryOperation,
        _span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }

    /// Visit comparison operation
    fn visit_comparison_operation(
        &mut self,
        op: &mut ComparisonOperation,
        _span: SimpleSpan,
    ) {
        op.visit_children_with(self);
    }

    /// Visit dereference assignment
    fn visit_deref_assignment(
        &mut self,
        deref_assign: &mut DerefAssignment,
        _span: SimpleSpan,
    ) {
        deref_assign.visit_children_with(self);
    }

    /// Visit apply chain
    fn visit_apply_chain(
        &mut self,
        apply_chain: &mut ApplyChain,
        _span: SimpleSpan,
    ) {
        apply_chain.visit_children_with(self);
    }

    /// Visit remote execution
    fn visit_remote_execution(
        &mut self,
        remote_execution: &mut RemoteExecution,
        _span: SimpleSpan,
    ) {
        remote_execution.visit_children_with(self);
    }

    /// Visit function declaration
    fn visit_function_declaration(
        &mut self,
        func_decl: &mut FunctionDeclaration,
        _span: SimpleSpan,
    ) {
        func_decl.visit_children_with(self);
    }

    /// Visit slot assignment
    fn visit_slot_assignment(
        &mut self,
        slot_assign: &mut SlotAssignment,
        _span: SimpleSpan,
    ) {
        slot_assign.visit_children_with(self);
    }

    /// Visit variable declaration
    fn visit_variable_declaration(
        &mut self,
        var_decl: &mut VariableDeclaration,
        _span: SimpleSpan,
    ) {
        var_decl.visit_children_with(self);
    }

    /// Visit variable assignment
    fn visit_variable_assignment(
        &mut self,
        var_assign: &mut VariableAssignment,
        _span: SimpleSpan,
    ) {
        var_assign.visit_children_with(self);
    }

    /// Visit variable access
    fn visit_variable_access(
        &mut self,
        _var_access: &mut VariableAccess,
        _span: SimpleSpan,
    ) {
    }

    /// Visit create reference expression
    fn visit_create_ref(
        &mut self,
        datex_expression: &mut DatexExpression,
        _span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }

    /// Visit create mutable reference expression
    fn visit_create_mut(
        &mut self,
        datex_expression: &mut DatexExpression,
        _span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }

    /// Visit dereference expression
    fn visit_deref(
        &mut self,
        datex_expression: &mut DatexExpression,
        _span: SimpleSpan,
    ) {
        datex_expression.visit_children_with(self);
    }

    /// Visit list expression
    fn visit_list(&mut self, list: &mut List, _span: SimpleSpan) {
        list.visit_children_with(self);
    }

    /// Visit map expression
    fn visit_map(&mut self, map: &mut Map, _span: SimpleSpan) {
        map.visit_children_with(self);
    }

    /// Visit integer literal
    fn visit_integer(&mut self, _value: &mut Integer, _span: SimpleSpan) {}

    /// Visit typed integer literal
    fn visit_typed_integer(
        &mut self,
        _value: &TypedInteger,
        _span: SimpleSpan,
    ) {
    }

    /// Visit decimal literal
    fn visit_decimal(&mut self, _value: &mut Decimal, _span: SimpleSpan) {}

    /// Visit typed decimal literal
    fn visit_typed_decimal(
        &mut self,
        _value: &TypedDecimal,
        _span: SimpleSpan,
    ) {
    }

    /// Visit text literal
    fn visit_text(&mut self, _value: &mut String, _span: SimpleSpan) {}

    /// Visit get reference expression
    fn visit_get_reference(
        &mut self,
        _pointer_address: &mut PointerAddress,
        _span: SimpleSpan,
    ) {
    }

    /// Visit boolean literal
    fn visit_boolean(&mut self, _value: &mut bool, _span: SimpleSpan) {}

    /// Visit endpoint expression
    fn visit_endpoint(&mut self, _value: &mut Endpoint, _span: SimpleSpan) {}

    /// Visit null literal
    fn visit_null(&mut self, _span: SimpleSpan) {}

    /// Visit pointer address expression
    fn visit_pointer_address(
        &mut self,
        _pointer_address: &PointerAddress,
        _span: SimpleSpan,
    ) {
    }

    /// Visit slot expression
    fn visit_slot(&mut self, _slot: &Slot, _span: SimpleSpan) {}
}
