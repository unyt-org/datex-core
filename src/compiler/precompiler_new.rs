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

pub struct Precompiler<'a> {
    options: PrecompilerOptions,
    ast: Option<&'a ValidDatexParseResult>,
    metadata: AstMetadata,
    scope_stack: PrecompilerScopeStack,
}

impl<'a> Precompiler<'a> {
    pub fn new(options: PrecompilerOptions) -> Self {
        Self {
            options,
            ast: None,
            metadata: AstMetadata::default(),
            scope_stack: PrecompilerScopeStack::default(),
        }
    }
    pub fn precompile(&mut self, ast: &'a mut ValidDatexParseResult) {
        self.metadata = AstMetadata::default();
        self.scope_stack = PrecompilerScopeStack::default();

        self.visit_expression(&mut ast.ast);
    }

    fn span(&self, span: SimpleSpan) -> Option<SimpleSpan> {
        let spans = &self.ast.unwrap().spans;
        // skip if both zero (default span used for testing)
        // TODO: improve this
        if span.start != 0 || span.end != 0 {
            let start_token = spans.get(span.start).cloned().unwrap();
            let end_token = spans.get(span.end - 1).cloned().unwrap();
            let full_span = start_token.start..end_token.end;
            Some(SimpleSpan::from(full_span))
        } else {
            None
        }
    }
}
impl Visit for Precompiler<'_> {
    fn visit_expression(&mut self, expression: &mut DatexExpression) {
        if let Some(span) = self.span(expression.span) {
            expression.span = span;
        }

        //println!("Visiting expression: {:?}", expr);
        // expr.visit_children_with(self);
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
