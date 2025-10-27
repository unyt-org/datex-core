use std::ops::Range;

use crate::ast::chain::ApplyOperation;
use crate::ast::data::expression::{
    ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
    DatexExpression, DatexExpressionData, DerefAssignment, FunctionDeclaration,
    List, Map, RemoteExecution, Slot, SlotAssignment, Statements,
    TypeDeclaration, UnaryOperation, VariableAccess, VariableAssignment,
    VariableDeclaration,
};
use crate::ast::data::r#type::{
    FixedSizeList, FunctionType, GenericAccess, Intersection, SliceList,
    StructuralList, StructuralMap, TypeExpression, TypeExpressionData, Union,
};
use crate::ast::visitor::VisitAction;
use crate::ast::visitor::expression::ExpressionVisitor;
use crate::ast::visitor::type_expression::TypeExpressionVisitor;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;

pub type TypeExpressionVisitAction = VisitAction<TypeExpression>;
pub trait VisitableTypeExpression {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor);
}

impl VisitableTypeExpression for StructuralList {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        for item in &mut self.0 {
            item.walk_children(visitor);
        }
    }
}
impl VisitableTypeExpression for FixedSizeList {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        self.r#type.walk_children(visitor);
    }
}
impl VisitableTypeExpression for SliceList {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        self.0.walk_children(visitor);
    }
}
impl VisitableTypeExpression for Intersection {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        for item in &mut self.0 {
            item.walk_children(visitor);
        }
    }
}
impl VisitableTypeExpression for Union {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        for item in &mut self.0 {
            item.walk_children(visitor);
        }
    }
}

impl VisitableTypeExpression for GenericAccess {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        for arg in &mut self.access {
            arg.walk_children(visitor);
        }
    }
}
impl VisitableTypeExpression for FunctionType {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        for (_, param_type) in &mut self.parameters {
            param_type.walk_children(visitor);
        }
        self.return_type.walk_children(visitor);
    }
}
impl VisitableTypeExpression for StructuralMap {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        for (_, value) in &mut self.0 {
            value.walk_children(visitor);
        }
    }
}

impl VisitableTypeExpression for TypeExpression {
    fn walk_children(&mut self, visitor: &mut impl TypeExpressionVisitor) {
        match &mut self.data {
            TypeExpressionData::StructuralList(structural_list) => {
                structural_list.walk_children(visitor)
            }
            TypeExpressionData::FixedSizeList(fixed_size_list) => {
                fixed_size_list.walk_children(visitor)
            }
            TypeExpressionData::SliceList(slice_list) => {
                slice_list.walk_children(visitor)
            }
            TypeExpressionData::Intersection(intersection) => {
                intersection.walk_children(visitor)
            }
            TypeExpressionData::Union(union) => union.walk_children(visitor),
            TypeExpressionData::GenericAccess(generic_access) => {
                generic_access.walk_children(visitor)
            }
            TypeExpressionData::Function(function_type) => {
                function_type.walk_children(visitor)
            }
            TypeExpressionData::StructuralMap(structural_map) => {
                structural_map.walk_children(visitor)
            }
            TypeExpressionData::Ref(type_expression) => {
                type_expression.walk_children(visitor)
            }
            TypeExpressionData::RefMut(type_expression) => {
                type_expression.walk_children(visitor)
            }
            TypeExpressionData::RefFinal(type_expression) => {
                type_expression.walk_children(visitor)
            }
            TypeExpressionData::Null
            | TypeExpressionData::Literal(_)
            | TypeExpressionData::VariableAccess(_)
            | TypeExpressionData::GetReference(_)
            | TypeExpressionData::Integer(_)
            | TypeExpressionData::TypedInteger(_)
            | TypeExpressionData::Decimal(_)
            | TypeExpressionData::TypedDecimal(_)
            | TypeExpressionData::Boolean(_)
            | TypeExpressionData::Text(_)
            | TypeExpressionData::Endpoint(_) => {}
        }
    }
}
