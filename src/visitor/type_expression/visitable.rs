use crate::ast::structs::r#type::{
    FixedSizeList, FunctionType, GenericAccess, Intersection, SliceList,
    StructuralList, StructuralMap, TypeExpression, TypeExpressionData, Union,
};
use crate::visitor::type_expression::TypeExpressionVisitor;
use crate::visitor::{ErrorWithVisitAction, VisitAction};

pub type TypeExpressionVisitAction<T: ErrorWithVisitAction<TypeExpression>> =
    Result<VisitAction<TypeExpression>, T>;

pub trait VisitableTypeExpression<T: ErrorWithVisitAction<TypeExpression>> {
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()>;
}

impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for StructuralList
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        for item in &mut self.0 {
            item.walk_children(visitor)?;
        }
        Ok(())
    }
}
impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for FixedSizeList
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        self.r#type.walk_children(visitor)
    }
}
impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for SliceList
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        self.0.walk_children(visitor)
    }
}
impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for Intersection
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        for item in &mut self.0 {
            item.walk_children(visitor)?;
        }
        Ok(())
    }
}
impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for Union
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        for item in &mut self.0 {
            item.walk_children(visitor)?;
        }
        Ok(())
    }
}

impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for GenericAccess
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        for arg in &mut self.access {
            arg.walk_children(visitor)?;
        }
        Ok(())
    }
}
impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for FunctionType
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        for (_, param_type) in &mut self.parameters {
            param_type.walk_children(visitor)?;
        }
        self.return_type.walk_children(visitor)?;
        Ok(())
    }
}
impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for StructuralMap
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
        for (_, value) in &mut self.0 {
            value.walk_children(visitor)?;
        }
        Ok(())
    }
}

impl<T: ErrorWithVisitAction<TypeExpression>> VisitableTypeExpression<T>
    for TypeExpression
{
    fn walk_children(
        &mut self,
        visitor: &mut impl TypeExpressionVisitor<T>,
    ) -> Result<(), ()> {
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
            | TypeExpressionData::Endpoint(_) => Ok(()),
        }
    }
}
