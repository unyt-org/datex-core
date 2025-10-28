use std::ops::Range;

use crate::ast::data::r#type::{
    FixedSizeList, FunctionType, GenericAccess, Intersection, SliceList,
    StructuralList, StructuralMap, TypeExpression, TypeExpressionData, Union,
};
use crate::ast::structs::expression::VariableAccess;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;
use crate::visitor::VisitAction;
use crate::visitor::type_expression::visitable::{
    TypeExpressionVisitAction, VisitableTypeExpression,
};
pub mod visitable;
pub trait TypeExpressionVisitor: Sized {
    fn visit_type_expression(&mut self, expr: &mut TypeExpression) {
        let action = match &mut expr.data {
            TypeExpressionData::GetReference(pointer_address) => {
                self.visit_get_reference_type(pointer_address, &expr.span)
            }
            TypeExpressionData::Null => self.visit_null_type(&expr.span),
            TypeExpressionData::VariableAccess(variable_access) => {
                self.visit_variable_access_type(variable_access, &expr.span)
            }
            TypeExpressionData::Integer(integer) => {
                self.visit_integer_type(integer, &expr.span)
            }
            TypeExpressionData::TypedInteger(typed_integer) => {
                self.visit_typed_integer_type(typed_integer, &expr.span)
            }
            TypeExpressionData::Decimal(decimal) => {
                self.visit_decimal_type(decimal, &expr.span)
            }
            TypeExpressionData::TypedDecimal(typed_decimal) => {
                self.visit_typed_decimal_type(typed_decimal, &expr.span)
            }
            TypeExpressionData::Boolean(boolean) => {
                self.visit_boolean_type(boolean, &expr.span)
            }
            TypeExpressionData::Text(text) => {
                self.visit_text_type(text, &expr.span)
            }
            TypeExpressionData::Endpoint(endpoint) => {
                self.visit_endpoint_type(endpoint, &expr.span)
            }
            TypeExpressionData::StructuralList(structual_list) => {
                self.visit_structural_list_type(structual_list, &expr.span)
            }
            TypeExpressionData::FixedSizeList(fixed_size_list) => {
                self.visit_fixed_size_list_type(fixed_size_list, &expr.span)
            }
            TypeExpressionData::SliceList(slice_list) => {
                self.visit_slice_list_type(slice_list, &expr.span)
            }
            TypeExpressionData::Intersection(intersection) => {
                self.visit_intersection_type(intersection, &expr.span)
            }
            TypeExpressionData::Union(union) => {
                self.visit_union_type(union, &expr.span)
            }
            TypeExpressionData::GenericAccess(generic_access) => {
                self.visit_generic_access_type(generic_access, &expr.span)
            }
            TypeExpressionData::Function(function) => {
                self.visit_function_type(function, &expr.span)
            }
            TypeExpressionData::StructuralMap(structural_map) => {
                self.visit_structural_map_type(structural_map, &expr.span)
            }
            TypeExpressionData::Ref(type_ref) => {
                self.visit_ref_type(type_ref, &expr.span)
            }
            TypeExpressionData::RefMut(type_ref_mut) => {
                self.visit_ref_mut_type(type_ref_mut, &expr.span)
            }
            TypeExpressionData::Literal(literal) => {
                self.visit_literal_type(literal, &expr.span)
            }
            TypeExpressionData::RefFinal(type_ref_final) => {
                unimplemented!("RefFinal is going to be deprecated")
            }
        };

        match action {
            VisitAction::SkipChildren => {}
            VisitAction::ToNoop => {
                expr.data = TypeExpressionData::Null;
            }
            VisitAction::VisitChildren => expr.walk_children(self),
            VisitAction::Replace(new_expr) => *expr = new_expr,
            VisitAction::ReplaceRecurseChildNodes(new_expr) => {
                expr.walk_children(self);
                *expr = new_expr;
            }
            VisitAction::ReplaceRecurse(new_expr) => {
                *expr = new_expr;
                self.visit_type_expression(expr);
            }
        }
    }

    /// Visit literal type expression
    fn visit_literal_type(
        &mut self,
        literal: &mut String,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = literal;
        TypeExpressionVisitAction::SkipChildren
    }

    /// Visit structural list type expression
    fn visit_structural_list_type(
        &mut self,
        structural_list: &mut StructuralList,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = structural_list;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit fixed size list type expression
    fn visit_fixed_size_list_type(
        &mut self,
        fixed_size_list: &mut FixedSizeList,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = fixed_size_list;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit slice list type expression
    fn visit_slice_list_type(
        &mut self,
        slice_list: &mut SliceList,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = slice_list;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit intersection type expression
    fn visit_intersection_type(
        &mut self,
        intersection: &mut Intersection,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = intersection;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit union type expression
    fn visit_union_type(
        &mut self,
        union: &mut Union,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = union;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit generic access type expression
    fn visit_generic_access_type(
        &mut self,
        generic_access: &mut GenericAccess,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = generic_access;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit function type expression
    fn visit_function_type(
        &mut self,
        function_type: &mut FunctionType,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = function_type;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit structural map type expression
    fn visit_structural_map_type(
        &mut self,
        structural_map: &mut StructuralMap,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = structural_map;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit type reference expression
    fn visit_ref_type(
        &mut self,
        type_ref: &mut TypeExpression,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = type_ref;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit mutable type reference expression
    fn visit_ref_mut_type(
        &mut self,
        type_ref_mut: &mut TypeExpression,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = type_ref_mut;
        TypeExpressionVisitAction::VisitChildren
    }

    /// Visit integer literal
    fn visit_integer_type(
        &mut self,
        integer: &mut Integer,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = integer;
        VisitAction::SkipChildren
    }

    /// Visit typed integer literal
    fn visit_typed_integer_type(
        &mut self,
        typed_integer: &TypedInteger,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = typed_integer;
        VisitAction::SkipChildren
    }

    /// Visit decimal literal
    fn visit_decimal_type(
        &mut self,
        decimal: &mut Decimal,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = decimal;
        VisitAction::SkipChildren
    }

    /// Visit typed decimal literal
    fn visit_typed_decimal_type(
        &mut self,
        typed_decimal: &TypedDecimal,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = typed_decimal;
        VisitAction::SkipChildren
    }

    /// Visit text literal
    fn visit_text_type(
        &mut self,
        text: &mut String,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = text;
        VisitAction::SkipChildren
    }

    /// Visit get reference expression
    fn visit_get_reference_type(
        &mut self,
        pointer_address: &mut PointerAddress,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = pointer_address;
        VisitAction::SkipChildren
    }

    /// Visit boolean literal
    fn visit_boolean_type(
        &mut self,
        boolean: &mut bool,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = boolean;
        VisitAction::SkipChildren
    }

    /// Visit endpoint expression
    fn visit_endpoint_type(
        &mut self,
        endpoint: &mut Endpoint,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = endpoint;
        VisitAction::SkipChildren
    }

    /// Visit null literal
    fn visit_null_type(
        &mut self,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        VisitAction::SkipChildren
    }

    /// Visit variable access
    fn visit_variable_access_type(
        &mut self,
        var_access: &mut VariableAccess,
        span: &Range<usize>,
    ) -> TypeExpressionVisitAction {
        let _ = span;
        let _ = var_access;
        VisitAction::SkipChildren
    }
}
