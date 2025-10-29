use crate::ast::structs::expression::{
    ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
    DatexExpression, DatexExpressionData, DerefAssignment, FunctionDeclaration,
    List, Map, RemoteExecution, SlotAssignment, Statements, TypeDeclaration,
    UnaryOperation, VariableAssignment, VariableDeclaration,
};
use crate::ast::structs::operator::ApplyOperation;
use crate::ast::structs::r#type::TypeExpression;
use crate::visitor::expression::ExpressionVisitor;
use crate::visitor::type_expression::visitable::VisitableTypeExpression;
use crate::visitor::{ErrorWithVisitAction, VisitAction};

pub type ExpressionVisitAction<T: ErrorWithVisitAction<DatexExpression>> =
    Result<VisitAction<DatexExpression>, T>;

pub trait VisitableExpression<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
>
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()>;
}

impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for BinaryOperation
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.left)?;
        visitor.visit_datex_expression(&mut self.right)?;
        Ok(())
    }
}

impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for Statements
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        for item in &mut self.statements {
            visitor.visit_datex_expression(item)?;
        }
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for List
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        for item in &mut self.items {
            visitor.visit_datex_expression(item)?;
        }
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for Map
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        for (_key, value) in &mut self.entries {
            visitor.visit_datex_expression(value)?;
        }
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for Conditional
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.condition)?;
        visitor.visit_datex_expression(&mut self.then_branch)?;
        if let Some(else_branch) = &mut self.else_branch {
            visitor.visit_datex_expression(else_branch)?;
        }
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for VariableDeclaration
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.init_expression)?;
        if let Some(type_annotation) = &mut self.r#type_annotation {
            visitor.visit_type_expression(type_annotation);
        }
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for VariableAssignment
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for UnaryOperation
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for TypeDeclaration
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_type_expression(&mut self.value)?;
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for ComparisonOperation
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.left)?;
        visitor.visit_datex_expression(&mut self.right)?;
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for DerefAssignment
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.assigned_expression)?;
        visitor.visit_datex_expression(&mut self.deref_expression)?;
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for ApplyChain
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.base);
        for operation in &mut self.operations {
            match operation {
                ApplyOperation::FunctionCall(arg) => {
                    visitor.visit_datex_expression(arg)?;
                }
                ApplyOperation::GenericAccess(arg) => {
                    visitor.visit_datex_expression(arg)?;
                }
                ApplyOperation::PropertyAccess(prop) => {
                    visitor.visit_datex_expression(prop)?;
                }
            }
        }
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for RemoteExecution
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.left)?;
        visitor.visit_datex_expression(&mut self.right)?;
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for SlotAssignment
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}
impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for FunctionDeclaration
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        for (_, param_type) in &mut self.parameters {
            visitor.visit_type_expression(param_type)?;
        }
        visitor.visit_datex_expression(&mut self.body)?;
        Ok(())
    }
}

impl<
    T: ErrorWithVisitAction<DatexExpression>,
    U: ErrorWithVisitAction<TypeExpression>,
> VisitableExpression<T, U> for DatexExpression
{
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<T, U>,
    ) -> Result<(), ()> {
        match &mut self.data {
            DatexExpressionData::BinaryOperation(op) => {
                op.walk_children(visitor)
            }
            DatexExpressionData::Statements(statements) => {
                statements.walk_children(visitor)
            }
            DatexExpressionData::List(list) => list.walk_children(visitor),
            DatexExpressionData::Map(map) => map.walk_children(visitor),
            DatexExpressionData::Conditional(conditional) => {
                conditional.walk_children(visitor)
            }
            DatexExpressionData::VariableDeclaration(variable_declaration) => {
                variable_declaration.walk_children(visitor)
            }
            DatexExpressionData::VariableAssignment(variable_assignment) => {
                variable_assignment.walk_children(visitor)
            }
            DatexExpressionData::TypeDeclaration(type_declaration) => {
                type_declaration.walk_children(visitor)
            }
            DatexExpressionData::TypeExpression(type_expression) => {
                type_expression.walk_children(visitor)
            }
            DatexExpressionData::Type(type_expression) => {
                type_expression.walk_children(visitor)
            }
            DatexExpressionData::FunctionDeclaration(function_declaration) => {
                function_declaration.walk_children(visitor)
            }
            DatexExpressionData::CreateRef(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::CreateRefMut(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::CreateRefFinal(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::Deref(datex_expression) => {
                datex_expression.walk_children(visitor)
            }
            DatexExpressionData::SlotAssignment(slot_assignment) => {
                slot_assignment.walk_children(visitor)
            }
            DatexExpressionData::ComparisonOperation(comparison_operation) => {
                comparison_operation.walk_children(visitor)
            }
            DatexExpressionData::DerefAssignment(deref_assignment) => {
                deref_assignment.walk_children(visitor)
            }
            DatexExpressionData::UnaryOperation(unary_operation) => {
                unary_operation.walk_children(visitor)
            }
            DatexExpressionData::ApplyChain(apply_chain) => {
                apply_chain.walk_children(visitor)
            }
            DatexExpressionData::RemoteExecution(remote_execution) => {
                remote_execution.walk_children(visitor)
            }

            DatexExpressionData::Noop
            | DatexExpressionData::PointerAddress(_)
            | DatexExpressionData::VariableAccess(_)
            | DatexExpressionData::GetReference(_)
            | DatexExpressionData::Slot(_)
            | DatexExpressionData::Placeholder
            | DatexExpressionData::Recover
            | DatexExpressionData::Null
            | DatexExpressionData::Boolean(_)
            | DatexExpressionData::Text(_)
            | DatexExpressionData::Decimal(_)
            | DatexExpressionData::TypedDecimal(_)
            | DatexExpressionData::Integer(_)
            | DatexExpressionData::TypedInteger(_)
            | DatexExpressionData::Identifier(_)
            | DatexExpressionData::Endpoint(_) => Ok(()),
        }
    }
}
