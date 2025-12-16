use crate::ast::structs::VariableId;
use crate::ast::structs::expression::{DatexExpression, VariableDeclaration};
use crate::visitor::VisitAction;
use crate::visitor::expression::ExpressionVisitor;
use crate::visitor::type_expression::TypeExpressionVisitor;
use core::ops::Range;

#[derive(Default)]
pub struct TypeHintCollector {
    pub type_hints: Vec<(usize, VariableId)>,
}

impl TypeExpressionVisitor<()> for TypeHintCollector {}

impl ExpressionVisitor<()> for TypeHintCollector {
    fn visit_variable_declaration(
        &mut self,
        var_decl: &mut VariableDeclaration,
        span: &Range<usize>,
    ) -> Result<VisitAction<DatexExpression>, ()> {
        if var_decl.type_annotation.is_none() {
            let expr_start = var_decl.init_expression.span.start;
            // TODO: improve
            self.type_hints.push((expr_start - 3, var_decl.id.unwrap()));
        }
        Ok(VisitAction::VisitChildren)
    }
}
