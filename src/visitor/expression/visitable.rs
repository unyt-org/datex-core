use crate::ast::structs::expression::{
    BinaryOperation, CallableDeclaration, ComparisonOperation, Conditional,
    CreateRef, DatexExpression, DatexExpressionData, Deref, DerefAssignment,
    GenericInstantiation, List, Map, PropertyAccess, PropertyAssignment,
    RemoteExecution, SlotAssignment, Statements, TypeDeclaration,
    UnaryOperation, VariableAssignment, VariableDeclaration,
};
use crate::visitor::VisitAction;
use crate::visitor::expression::ExpressionVisitor;
use crate::visitor::type_expression::visitable::VisitableTypeExpression;
use datex_core::ast::structs::expression::Apply;

pub type ExpressionVisitResult<E> = Result<VisitAction<DatexExpression>, E>;

pub trait VisitableExpression<E> {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E>;
}

impl<E> VisitableExpression<E> for BinaryOperation {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.left)?;
        visitor.visit_datex_expression(&mut self.right)?;
        Ok(())
    }
}

impl<E> VisitableExpression<E> for Statements {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        for item in &mut self.statements {
            visitor.visit_datex_expression(item)?;
        }
        Ok(())
    }
}
impl<E> VisitableExpression<E> for List {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        for item in &mut self.items {
            visitor.visit_datex_expression(item)?;
        }
        Ok(())
    }
}
impl<E> VisitableExpression<E> for Map {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        for (_key, value) in &mut self.entries {
            visitor.visit_datex_expression(value)?;
        }
        Ok(())
    }
}
impl<E> VisitableExpression<E> for Conditional {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.condition)?;
        visitor.visit_datex_expression(&mut self.then_branch)?;
        if let Some(else_branch) = &mut self.else_branch {
            visitor.visit_datex_expression(else_branch)?;
        }
        Ok(())
    }
}
impl<E> VisitableExpression<E> for VariableDeclaration {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        //visitor.visit_identifier(&mut self.name, self.)?;
        visitor.visit_datex_expression(&mut self.init_expression)?;
        if let Some(type_annotation) = &mut self.type_annotation {
            visitor.visit_type_expression(type_annotation)?;
        }
        Ok(())
    }
}
impl<E> VisitableExpression<E> for VariableAssignment {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}
impl<E> VisitableExpression<E> for UnaryOperation {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}
impl<E> VisitableExpression<E> for TypeDeclaration {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_type_expression(&mut self.definition)?;
        Ok(())
    }
}
impl<E> VisitableExpression<E> for ComparisonOperation {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.left)?;
        visitor.visit_datex_expression(&mut self.right)?;
        Ok(())
    }
}
impl<E> VisitableExpression<E> for DerefAssignment {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.assigned_expression)?;
        visitor.visit_datex_expression(&mut self.deref_expression)?;
        Ok(())
    }
}
impl<E> VisitableExpression<E> for Apply {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.base)?;
        for arg in &mut self.arguments {
            visitor.visit_datex_expression(arg)?;
        }
        Ok(())
    }
}

impl<E> VisitableExpression<E> for PropertyAccess {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.base)?;
        visitor.visit_datex_expression(&mut self.property)?;
        Ok(())
    }
}

impl<E> VisitableExpression<E> for GenericInstantiation {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.base)?;
        for arg in &mut self.generic_arguments {
            visitor.visit_type_expression(arg)?;
        }
        Ok(())
    }
}

impl<E> VisitableExpression<E> for RemoteExecution {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.left)?;
        visitor.visit_datex_expression(&mut self.right)?;
        Ok(())
    }
}
impl<E> VisitableExpression<E> for SlotAssignment {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}
impl<E> VisitableExpression<E> for CallableDeclaration {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        if let Some(return_type) = &mut self.return_type {
            visitor.visit_type_expression(return_type)?;
        }
        for (_, param_type) in &mut self.parameters {
            visitor.visit_type_expression(param_type)?;
        }
        visitor.visit_datex_expression(&mut self.body)?;
        Ok(())
    }
}

impl<E> VisitableExpression<E> for Deref {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}

impl<E> VisitableExpression<E> for CreateRef {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.expression)?;
        Ok(())
    }
}

impl<E> VisitableExpression<E> for PropertyAssignment {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        visitor.visit_datex_expression(&mut self.access_expression)?;
        visitor.visit_datex_expression(&mut self.assigned_expression)?;
        Ok(())
    }
}

impl<E> VisitableExpression<E> for DatexExpression {
    fn walk_children(
        &mut self,
        visitor: &mut impl ExpressionVisitor<E>,
    ) -> Result<(), E> {
        match &mut self.data {
            DatexExpressionData::PropertyAssignment(property_assignment) => {
                property_assignment.walk_children(visitor)
            }
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
            DatexExpressionData::CallableDeclaration(function_declaration) => {
                function_declaration.walk_children(visitor)
            }
            DatexExpressionData::CreateRef(create_ref) => {
                create_ref.walk_children(visitor)
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
            DatexExpressionData::Apply(apply_chain) => {
                apply_chain.walk_children(visitor)
            }
            DatexExpressionData::PropertyAccess(property_access) => {
                property_access.walk_children(visitor)
            }
            DatexExpressionData::GenericInstantiation(
                generic_instantiation,
            ) => generic_instantiation.walk_children(visitor),
            DatexExpressionData::RemoteExecution(remote_execution) => {
                remote_execution.walk_children(visitor)
            }

            DatexExpressionData::Noop
            | DatexExpressionData::VariantAccess(_)
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
