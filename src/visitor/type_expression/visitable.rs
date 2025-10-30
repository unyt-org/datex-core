use crate::ast::structs::r#type::{
    FixedSizeList, FunctionType, GenericAccess, Intersection, SliceList,
    StructuralList, StructuralMap, TypeExpression, TypeExpressionData, Union,
};
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
            item.walk_children(visitor)?;
        }
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for FixedSizeList {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        self.r#type.walk_children(visitor)
    }
}
impl<E> VisitableTypeExpression<E> for SliceList {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        self.0.walk_children(visitor)
    }
}
impl<E> VisitableTypeExpression<E> for Intersection {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for item in &mut self.0 {
            item.walk_children(visitor)?;
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
            item.walk_children(visitor)?;
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
            arg.walk_children(visitor)?;
        }
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for FunctionType {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for (_, param_type) in &mut self.parameters {
            param_type.walk_children(visitor)?;
        }
        self.return_type.walk_children(visitor)?;
        Ok(())
    }
}
impl<E> VisitableTypeExpression<E> for StructuralMap {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<E>,
    ) -> Result<(), E> {
        for (_, value) in &mut self.0 {
            value.walk_children(visitor)?;
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
            | TypeExpressionData::VariantAccess(_)
            | TypeExpressionData::Endpoint(_) => Ok(()),
        }
    }
}
