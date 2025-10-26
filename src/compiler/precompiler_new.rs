use std::ops::Range;

use chumsky::span::SimpleSpan;
use uuid::fmt::Simple;

use crate::{
    ast::{
        self,
        data::{
            expression::DatexExpression,
            r#type::TypeExpression,
            visitor::{Visit, Visitable},
        },
        parse_result::ValidDatexParseResult,
    },
    compiler::precompiler::{
        AstMetadata, PrecompilerOptions, PrecompilerScopeStack,
    },
};

pub struct Precompiler {
    options: PrecompilerOptions,
    spans: Vec<Range<usize>>,
    metadata: AstMetadata,
    scope_stack: PrecompilerScopeStack,
}

impl Precompiler {
    pub fn new(options: PrecompilerOptions) -> Self {
        Self {
            options,
            spans: Vec::new(),
            metadata: AstMetadata::default(),
            scope_stack: PrecompilerScopeStack::default(),
        }
    }
    pub fn precompile(&mut self, ast: &mut ValidDatexParseResult) {
        self.metadata = AstMetadata::default();
        self.scope_stack = PrecompilerScopeStack::default();
        self.spans = ast.spans.clone();

        self.visit_expression(&mut ast.ast);
    }

    fn span(&self, span: SimpleSpan) -> Option<SimpleSpan> {
        // skip if both zero (default span used for testing)
        // TODO: improve this
        if span.start != 0 || span.end != 0 {
            let start_token = self.spans.get(span.start).cloned().unwrap();
            let end_token = self.spans.get(span.end - 1).cloned().unwrap();
            let full_span = start_token.start..end_token.end;
            Some(SimpleSpan::from(full_span))
        } else {
            None
        }
    }
}
impl Visit for Precompiler {
    fn visit_expression(&mut self, expression: &mut DatexExpression) {
        if let Some(span) = self.span(expression.span) {
            expression.span = span;
        }
        println!("Visiting expression: {:?}", expression);
		expression.visit_children_with(self);
    }
    fn visit_type_expression(&mut self, type_expr: &mut TypeExpression) {
        if let Some(span) = self.span(type_expr.span) {
            type_expr.span = span;
        }
        println!("Visiting type expression: {:?}", type_expr);
		type_expr.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::parse;

    use super::*;
    #[test]
    fn test_precompiler_visit() {
        let options = PrecompilerOptions::default();
        let mut precompiler = Precompiler::new(options);
        let mut ast = parse("var x: integer = 34;").unwrap();
        precompiler.precompile(&mut ast);
    }
}
