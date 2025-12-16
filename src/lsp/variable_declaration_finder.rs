use crate::ast::structs::expression::{DatexExpression, VariableDeclaration};
use crate::visitor::VisitAction;
use crate::visitor::expression::ExpressionVisitor;
use crate::visitor::type_expression::TypeExpressionVisitor;
use core::ops::Range;

#[derive(Default)]
pub struct VariableDeclarationFinder {
    pub var_id: usize,
    pub variable_declaration_position: Option<Range<usize>>,
}

impl VariableDeclarationFinder {
    pub fn new(var_id: usize) -> Self {
        VariableDeclarationFinder {
            var_id,
            variable_declaration_position: None,
        }
    }
}

impl TypeExpressionVisitor<()> for VariableDeclarationFinder {}

impl ExpressionVisitor<()> for VariableDeclarationFinder {
    fn visit_variable_declaration(
        &mut self,
        var_decl: &mut VariableDeclaration,
        span: &Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        if var_decl.id == Some(self.var_id) {
            self.variable_declaration_position = Some(span.clone());
            // early abort
            Err(())
        } else {
            Ok(VisitAction::VisitChildren)
        }
    }
}
