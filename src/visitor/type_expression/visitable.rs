use crate::ast::type_expressions::{CallableTypeExpression, FixedSizeList, GenericAccess, Intersection, SliceList, StructuralList, StructuralMap, TypeExpression, TypeExpressionData, Union};
use crate::visitor::VisitAction;
use crate::visitor::type_expression::TypeExpressionVisitor;

pub type TypeExpressionVisitResult<E> = Result<VisitAction<TypeExpression>, E>;

pub trait VisitableTypeExpression<E> {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E>;
}

impl<E> VisitableTypeExpression<E> for StructuralList {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for item in &mut self.0 {
            visitor.visit_type_expression(item)?;
        }
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for FixedSizeList {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_type_expression(&mut self.ty)?;
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for SliceList {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_type_expression(&mut self.0)?;
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for Intersection {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for item in &mut self.0 {
            visitor.visit_type_expression(item)?;
        }
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for Union {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for item in &mut self.0 {
            visitor.visit_type_expression(item)?;
        }
        Ok(())
    }
}

impl<E> VisitableTypeExpression<E> for GenericAccess {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for arg in &mut self.access {
            visitor.visit_type_expression(arg)?;
        }
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for CallableTypeExpression {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for (_, param_type) in &mut self.parameter_types {
            visitor.visit_type_expression(param_type)?;
        }
        if let Some((_, rest_param_type)) = &mut self.rest_parameter_type {
            visitor.visit_type_expression(rest_param_type)?;
        }
        if let Some(return_type) = &mut self.return_type {
            visitor.visit_type_expression(return_type)?;
        }
        if let Some(yeet_type) = &mut self.yeet_type {
            visitor.visit_type_expression(yeet_type)?;
        }
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for StructuralMap {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for (_, value) in &mut self.0 {
            visitor.visit_type_expression(value)?;
        }
        Ok(())
    }
}

impl<E> VisitableTypeExpression<E> for TypeExpression {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
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
            TypeExpressionData::Callable(callable_type_expression) => {
                callable_type_expression.walk_children(visitor)
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

            TypeExpressionData::Recover
            | TypeExpressionData::Null
            | TypeExpressionData::Unit
            | TypeExpressionData::Identifier(_)
            | TypeExpressionData::VariableAccess(_)
            | TypeExpressionData::GetReference(_)
            | TypeExpressionData::Integer(_)
            | TypeExpressionData::TypedInteger(_)
            | TypeExpressionData::Decimal(_)
            | TypeExpressionData::TypedDecimal(_)
            | TypeExpressionData::Boolean(_)
            | TypeExpressionData::Text(_)
            | TypeExpressionData::VariantAccess(_)
            | TypeExpressionData::Endpoint(_) => Ok(()),
        }
    }
}
