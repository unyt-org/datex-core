use core::ops::Range;
use datex_core::ast::structs::VariableId;
use datex_core::ast::structs::expression::{
    DatexExpression, VariableDeclaration,
};
use datex_core::visitor::VisitAction;
use datex_core::visitor::expression::ExpressionVisitor;
use datex_core::visitor::type_expression::TypeExpressionVisitor;

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
