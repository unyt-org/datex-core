use std::ops::Range;

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
    fn visit_children_mut_with(&mut self, visitor: &mut impl VisitMut);
    fn visit_children_with(&self, visitor: &mut impl Visit);
}

/// Visitor pattern for traversing the AST
/// Implement the `Visit` trait and override the methods for the nodes you want to visit.
/// The default implementation visits all child nodes and traverses the entire tree.
pub trait VisitMut: Sized {
    // Type Expressions
    fn visit_type_expression(&mut self, type_expr: &mut TypeExpression) {
        type_expr.visit_children_mut_with(self);
    }

    /// Visit literal type expression
    fn visit_literal_type(
        &mut self,
        _literal: &mut String,
        _span: &Range<usize>,
    ) {
    }

    /// Visit structural list type expression
    fn visit_structural_list(
        &mut self,
        structural_list: &mut StructuralList,
        _span: &Range<usize>,
    ) {
        structural_list.visit_children_mut_with(self);
    }

    /// Visit fixed size list type expression
    fn visit_fixed_size_list(
        &mut self,
        fixed_size_list: &mut FixedSizeList,
        _span: &Range<usize>,
    ) {
        fixed_size_list.visit_children_mut_with(self);
    }

    /// Visit slice list type expression
    fn visit_slice_list(
        &mut self,
        slice_list: &mut SliceList,
        _span: &Range<usize>,
    ) {
        slice_list.visit_children_mut_with(self);
    }

    /// Visit intersection type expression
    fn visit_intersection(
        &mut self,
        intersection: &mut Intersection,
        _span: &Range<usize>,
    ) {
        intersection.visit_children_mut_with(self);
    }

    /// Visit union type expression
    fn visit_union(&mut self, union: &mut Union, _span: &Range<usize>) {
        union.visit_children_mut_with(self);
    }

    /// Visit generic access type expression
    fn visit_generic_access(
        &mut self,
        generic_access: &mut GenericAccess,
        _span: &Range<usize>,
    ) {
        generic_access.visit_children_mut_with(self);
    }

    /// Visit function type expression
    fn visit_function_type(
        &mut self,
        function_type: &mut FunctionType,
        _span: &Range<usize>,
    ) {
        function_type.visit_children_mut_with(self);
    }

    /// Visit structural map type expression
    fn visit_structural_map(
        &mut self,
        structural_map: &mut StructuralMap,
        _span: &Range<usize>,
    ) {
        structural_map.visit_children_mut_with(self);
    }

    /// Visit type reference expression
    fn visit_type_ref(
        &mut self,
        type_ref: &mut TypeExpression,
        _span: &Range<usize>,
    ) {
        type_ref.visit_children_mut_with(self);
    }

    /// Visit mutable type reference expression
    fn visit_type_ref_mut(
        &mut self,
        type_ref_mut: &mut TypeExpression,
        _span: &Range<usize>,
    ) {
        type_ref_mut.visit_children_mut_with(self);
    }

    // Expressions

    /// Visit datex expression
    fn visit_expression(&mut self, expr: &mut DatexExpression) {
        expr.visit_children_mut_with(self);
    }

    /// Visit statements
    fn visit_statements(
        &mut self,
        stmts: &mut Statements,
        _span: &Range<usize>,
    ) {
        stmts.visit_children_mut_with(self);
    }

    /// Visit unary operation
    fn visit_unary_operation(
        &mut self,
        op: &mut UnaryOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_mut_with(self);
    }

    /// Visit conditional expression
    fn visit_conditional(
        &mut self,
        cond: &mut Conditional,
        _span: &Range<usize>,
    ) {
        cond.visit_children_mut_with(self);
    }

    /// Visit type declaration
    fn visit_type_declaration(
        &mut self,
        type_decl: &mut TypeDeclaration,
        _span: &Range<usize>,
    ) {
        type_decl.visit_children_mut_with(self);
    }

    /// Visit binary operation
    fn visit_binary_operation(
        &mut self,
        op: &mut BinaryOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_mut_with(self);
    }

    /// Visit comparison operation
    fn visit_comparison_operation(
        &mut self,
        op: &mut ComparisonOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_mut_with(self);
    }

    /// Visit dereference assignment
    fn visit_deref_assignment(
        &mut self,
        deref_assign: &mut DerefAssignment,
        _span: &Range<usize>,
    ) {
        deref_assign.visit_children_mut_with(self);
    }

    /// Visit apply chain
    fn visit_apply_chain(
        &mut self,
        apply_chain: &mut ApplyChain,
        _span: &Range<usize>,
    ) {
        apply_chain.visit_children_mut_with(self);
    }

    /// Visit remote execution
    fn visit_remote_execution(
        &mut self,
        remote_execution: &mut RemoteExecution,
        _span: &Range<usize>,
    ) {
        remote_execution.visit_children_mut_with(self);
    }

    /// Visit function declaration
    fn visit_function_declaration(
        &mut self,
        func_decl: &mut FunctionDeclaration,
        _span: &Range<usize>,
    ) {
        func_decl.visit_children_mut_with(self);
    }

    /// Visit slot assignment
    fn visit_slot_assignment(
        &mut self,
        slot_assign: &mut SlotAssignment,
        _span: &Range<usize>,
    ) {
        slot_assign.visit_children_mut_with(self);
    }

    /// Visit variable declaration
    fn visit_variable_declaration(
        &mut self,
        var_decl: &mut VariableDeclaration,
        _span: &Range<usize>,
    ) {
        var_decl.visit_children_mut_with(self);
    }

    /// Visit variable assignment
    fn visit_variable_assignment(
        &mut self,
        var_assign: &mut VariableAssignment,
        _span: &Range<usize>,
    ) {
        var_assign.visit_children_mut_with(self);
    }

    /// Visit variable access
    fn visit_variable_access(
        &mut self,
        _var_access: &mut VariableAccess,
        _span: &Range<usize>,
    ) {
    }

    /// Visit create reference expression
    fn visit_create_ref(
        &mut self,
        datex_expression: &mut DatexExpression,
        _span: &Range<usize>,
    ) {
        datex_expression.visit_children_mut_with(self);
    }

    /// Visit create mutable reference expression
    fn visit_create_mut(
        &mut self,
        datex_expression: &mut DatexExpression,
        _span: &Range<usize>,
    ) {
        datex_expression.visit_children_mut_with(self);
    }

    /// Visit dereference expression
    fn visit_deref(
        &mut self,
        datex_expression: &mut DatexExpression,
        _span: &Range<usize>,
    ) {
        datex_expression.visit_children_mut_with(self);
    }

    /// Visit list expression
    fn visit_list(&mut self, list: &mut List, _span: &Range<usize>) {
        list.visit_children_mut_with(self);
    }

    /// Visit map expression
    fn visit_map(&mut self, map: &mut Map, _span: &Range<usize>) {
        map.visit_children_mut_with(self);
    }

    /// Visit integer literal
    fn visit_integer(&mut self, _value: &mut Integer, _span: &Range<usize>) {}

    /// Visit typed integer literal
    fn visit_typed_integer(
        &mut self,
        _value: &mut TypedInteger,
        _span: &Range<usize>,
    ) {
    }

    /// Visit decimal literal
    fn visit_decimal(&mut self, _value: &mut Decimal, _span: &Range<usize>) {}

    /// Visit typed decimal literal
    fn visit_typed_decimal(
        &mut self,
        _value: &mut TypedDecimal,
        _span: &Range<usize>,
    ) {
    }

    /// Visit identifier
    fn visit_identifier(&mut self, _value: &mut String, _span: &Range<usize>) {}

    /// Visit text literal
    fn visit_text(&mut self, _value: &mut String, _span: &Range<usize>) {}

    /// Visit get reference expression
    fn visit_get_reference(
        &mut self,
        _pointer_address: &mut PointerAddress,
        _span: &Range<usize>,
    ) {
    }

    /// Visit boolean literal
    fn visit_boolean(&mut self, _value: &mut bool, _span: &Range<usize>) {}

    /// Visit endpoint expression
    fn visit_endpoint(&mut self, _value: &mut Endpoint, _span: &Range<usize>) {}

    /// Visit null literal
    fn visit_null(&mut self, _span: &Range<usize>) {}

    /// Visit pointer address expression
    fn visit_pointer_address(
        &mut self,
        _pointer_address: &PointerAddress,
        _span: &Range<usize>,
    ) {
    }

    /// Visit slot expression
    fn visit_slot(&mut self, _slot: &Slot, _span: &Range<usize>) {}
}

pub trait Visit: Sized {
    // Type Expressions
    fn visit_type_expression(&mut self, type_expr: &TypeExpression) {
        type_expr.visit_children_with(self);
    }

    /// Visit literal type expression
    fn visit_literal_type(
        &mut self,
        _literal: &String,
        _span: &Range<usize>,
    ){
    }

    /// Visit structural list type expression
    fn visit_structural_list(
        &mut self,
        structural_list: &StructuralList,
        _span: &Range<usize>,
    ) {
        structural_list.visit_children_with(self);
    }

    /// Visit fixed size list type expression
    fn visit_fixed_size_list(
        &mut self,
        fixed_size_list: &FixedSizeList,
        _span: &Range<usize>,
    ) {
        fixed_size_list.visit_children_with(self);
    }

    /// Visit slice list type expression
    fn visit_slice_list(
        &mut self,
        slice_list: &SliceList,
        _span: &Range<usize>,
    ) {
        slice_list.visit_children_with(self);
    }

    /// Visit intersection type expression
    fn visit_intersection(
        &mut self,
        intersection: &Intersection,
        _span: &Range<usize>,
    ) {
        intersection.visit_children_with(self);
    }

    /// Visit union type expression
    fn visit_union(&mut self, union: &Union, _span: &Range<usize>) {
        union.visit_children_with(self);
    }

    /// Visit generic access type expression
    fn visit_generic_access(
        &mut self,
        generic_access: &GenericAccess,
        _span: &Range<usize>,
    ) {
        generic_access.visit_children_with(self);
    }

    /// Visit function type expression
    fn visit_function_type(
        &mut self,
        function_type: &FunctionType,
        _span: &Range<usize>,
    ) {
        function_type.visit_children_with(self);
    }

    /// Visit structural map type expression
    fn visit_structural_map(
        &mut self,
        structural_map: &StructuralMap,
        _span: &Range<usize>,
    ) {
        structural_map.visit_children_with(self);
    }

    /// Visit type reference expression
    fn visit_type_ref(
        &mut self,
        type_ref: &TypeExpression,
        _span: &Range<usize>,
    ) {
        type_ref.visit_children_with(self);
    }

    /// Visit mutable type reference expression
    fn visit_type_ref_mut(
        &mut self,
        type_ref_mut: &TypeExpression,
        _span: &Range<usize>,
    ) {
        type_ref_mut.visit_children_with(self);
    }

    // Expressions

    /// Visit datex expression
    fn visit_expression(&mut self, expr: &DatexExpression) {
        expr.visit_children_with(self);
    }

    /// Visit statements
    fn visit_statements(
        &mut self,
        stmts: &Statements,
        _span: &Range<usize>,
    ) {
        stmts.visit_children_with(self);
    }

    /// Visit unary operation
    fn visit_unary_operation(
        &mut self,
        op: &UnaryOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_with(self);
    }

    /// Visit conditional expression
    fn visit_conditional(
        &mut self,
        cond: &Conditional,
        _span: &Range<usize>,
    ) {
        cond.visit_children_with(self);
    }

    /// Visit type declaration
    fn visit_type_declaration(
        &mut self,
        type_decl: &TypeDeclaration,
        _span: &Range<usize>,
    ) {
        type_decl.visit_children_with(self);
    }

    /// Visit binary operation
    fn visit_binary_operation(
        &mut self,
        op: &BinaryOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_with(self);
    }

    /// Visit comparison operation
    fn visit_comparison_operation(
        &mut self,
        op: &ComparisonOperation,
        _span: &Range<usize>,
    ) {
        op.visit_children_with(self);
    }

    /// Visit dereference assignment
    fn visit_deref_assignment(
        &mut self,
        deref_assign: &DerefAssignment,
        _span: &Range<usize>,
    ) {
        deref_assign.visit_children_with(self);
    }

    /// Visit apply chain
    fn visit_apply_chain(
        &mut self,
        apply_chain: &ApplyChain,
        _span: &Range<usize>,
    ) {
        apply_chain.visit_children_with(self);
    }

    /// Visit remote execution
    fn visit_remote_execution(
        &mut self,
        remote_execution: &RemoteExecution,
        _span: &Range<usize>,
    ) {
        remote_execution.visit_children_with(self);
    }

    /// Visit function declaration
    fn visit_function_declaration(
        &mut self,
        func_decl: &FunctionDeclaration,
        _span: &Range<usize>,
    ) {
        func_decl.visit_children_with(self);
    }

    /// Visit slot assignment
    fn visit_slot_assignment(
        &mut self,
        slot_assign: &SlotAssignment,
        _span: &Range<usize>,
    ) {
        slot_assign.visit_children_with(self);
    }

    /// Visit variable declaration
    fn visit_variable_declaration(
        &mut self,
        var_decl: &VariableDeclaration,
        _span: &Range<usize>,
    ) {
        var_decl.visit_children_with(self);
    }

    /// Visit variable assignment
    fn visit_variable_assignment(
        &mut self,
        var_assign: &VariableAssignment,
        _span: &Range<usize>,
    ) {
        var_assign.visit_children_with(self);
    }

    /// Visit variable access
    fn visit_variable_access(
        &mut self,
        _var_access: &VariableAccess,
        _span: &Range<usize>,
    ) {
    }

    /// Visit create reference expression
    fn visit_create_ref(
        &mut self,
        datex_expression: &DatexExpression,
        _span: &Range<usize>,
    ) {
        datex_expression.visit_children_with(self);
    }

    /// Visit create mutable reference expression
    fn visit_create_mut(
        &mut self,
        datex_expression: &DatexExpression,
        _span: &Range<usize>,
    ) {
        datex_expression.visit_children_with(self);
    }

    /// Visit dereference expression
    fn visit_deref(
        &mut self,
        datex_expression: &DatexExpression,
        _span: &Range<usize>,
    ) {
        datex_expression.visit_children_with(self);
    }

    /// Visit list expression
    fn visit_list(&mut self, list: &List, _span: &Range<usize>) {
        list.visit_children_with(self);
    }

    /// Visit map expression
    fn visit_map(&mut self, map: &Map, _span: &Range<usize>) {
        map.visit_children_with(self);
    }

    /// Visit integer literal
    fn visit_integer(&mut self, _value: &Integer, _span: &Range<usize>) {}

    /// Visit typed integer literal
    fn visit_typed_integer(
        &mut self,
        _value: &TypedInteger,
        _span: &Range<usize>,
    ) {
    }

    /// Visit decimal literal
    fn visit_decimal(&mut self, _value: &Decimal, _span: &Range<usize>) {}

    /// Visit typed decimal literal
    fn visit_typed_decimal(
        &mut self,
        _value: &TypedDecimal,
        _span: &Range<usize>,
    ) {
    }

    /// Visit identifier
    fn visit_identifier(&mut self, _value: &String, _span: &Range<usize>) {}

    /// Visit text literal
    fn visit_text(&mut self, _value: &String, _span: &Range<usize>) {}

    /// Visit get reference expression
    fn visit_get_reference(
        &mut self,
        _pointer_address: &PointerAddress,
        _span: &Range<usize>,
    ) {
    }

    /// Visit boolean literal
    fn visit_boolean(&mut self, _value: &bool, _span: &Range<usize>) {}

    /// Visit endpoint expression
    fn visit_endpoint(&mut self, _value: &Endpoint, _span: &Range<usize>) {}

    /// Visit null literal
    fn visit_null(&mut self, _span: &Range<usize>) {}

    /// Visit pointer address expression
    fn visit_pointer_address(
        &mut self,
        _pointer_address: &PointerAddress,
        _span: &Range<usize>,
    ) {
    }

    /// Visit slot expression
    fn visit_slot(&mut self, _slot: &Slot, _span: &Range<usize>) {}
}
