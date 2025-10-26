use std::ops::Range;

use chumsky::span::SimpleSpan;
use log::info;
use uuid::fmt::Simple;

use crate::{
    ast::{
        self,
        data::{
            expression::{
                DatexExpression, DatexExpressionData, TypeDeclaration,
                VariableAccess, VariableDeclaration,
            },
            spanned::Spanned,
            r#type::TypeExpression,
            visitor::{self, Visit, Visitable},
        },
        parse_result::ValidDatexParseResult,
    },
    compiler::{
        error::{
            CompilerError, DetailedCompilerErrors, MaybeAction,
            SpannedCompilerError, collect_or_pass_error,
        },
        precompiler::{
            AstMetadata, PrecompilerOptions, PrecompilerScopeStack,
            VariableShape,
        },
    },
    libs::core::CoreLibPointerId,
    values::{pointer::PointerAddress, value_container::ValueContainer},
};

pub struct Precompiler {
    options: PrecompilerOptions,
    spans: Vec<Range<usize>>,
    metadata: AstMetadata,
    scope_stack: PrecompilerScopeStack,
    errors: Option<DetailedCompilerErrors>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ResolvedVariable {
    VariableId(usize),
    PointerAddress(PointerAddress),
}

impl Precompiler {
    pub fn new(options: PrecompilerOptions) -> Self {
        Self {
            options,
            spans: Vec::new(),
            metadata: AstMetadata::default(),
            scope_stack: PrecompilerScopeStack::default(),
            errors: None,
        }
    }
    pub fn precompile(&mut self, ast: &mut ValidDatexParseResult) {
        self.metadata = AstMetadata::default();
        self.scope_stack = PrecompilerScopeStack::default();
        self.spans = ast.spans.clone();

        self.errors = if self.options.detailed_errors {
            Some(DetailedCompilerErrors::default())
        } else {
            None
        };

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

    fn add_new_variable(&mut self, name: String, kind: VariableShape) -> usize {
        let new_id = self.metadata.variables.len();
        let var_metadata =
            self.scope_stack
                .add_new_variable(name.clone(), new_id, kind);
        self.metadata.variables.push(var_metadata);
        new_id
    }

    /// Resolves a variable name to either a local variable ID if it was already declared (or hoisted),
    /// or to a core library pointer ID if it is a core variable.
    /// If the variable cannot be resolved, a CompilerError is returned.
    fn resolve_variable(
        &mut self,
        name: &str,
    ) -> Result<ResolvedVariable, CompilerError> {
        // If variable exist
        if let Ok(id) = self
            .scope_stack
            .get_variable_and_update_metadata(name, &mut self.metadata)
        {
            info!("Visiting variable: {name}");
            Ok(ResolvedVariable::VariableId(id))
        }
        // try to resolve core variable
        else if let Some(core) = self.metadata
        .runtime
        .memory()
        .borrow()
        .get_reference(&CoreLibPointerId::Core.into()) // FIXME #444: don't use core struct here, but better access with one of our mappings already present
        && let Some(core_variable) = core
            .collapse_to_value()
            .borrow()
            .cast_to_map()
            .unwrap()
            .get_owned(name)
        {
            match core_variable {
                ValueContainer::Reference(reference) => {
                    if let Some(pointer_id) = reference.pointer_address() {
                        Ok(ResolvedVariable::PointerAddress(pointer_id))
                    } else {
                        unreachable!(
                            "Core variable reference must have a pointer ID"
                        );
                    }
                }
                _ => {
                    unreachable!("Core variable must be a reference");
                }
            }
        } else {
            Err(CompilerError::UndeclaredVariable(name.to_string()))
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

    fn visit_variable_declaration(
        &mut self,
        var_decl: &mut VariableDeclaration,
        _span: SimpleSpan,
    ) {
        var_decl.id = Some(self.add_new_variable(
            var_decl.name.clone(),
            VariableShape::Value(var_decl.kind),
        ));
        var_decl.visit_children_with(self);
    }

    fn visit_identifier(&mut self, name: &mut String, span: SimpleSpan) {
        let result = self.resolve_variable(name).map_err(|error| {
            SpannedCompilerError::new_with_simple_span(error, span)
        });
        let action = collect_or_pass_error(&mut self.errors, result).unwrap(); // FIXME: handle error properly
        if let MaybeAction::Do(resolved_variable) = action {
            let expression = match resolved_variable {
                ResolvedVariable::VariableId(id) => {
                    DatexExpressionData::VariableAccess(VariableAccess {
                        id,
                        name: name.clone(),
                    })
                    .with_span(span)
                }
                ResolvedVariable::PointerAddress(pointer_address) => {
                    DatexExpressionData::GetReference(pointer_address)
                        .with_span(span)
                }
            };
        }
    }

    fn visit_type_declaration(
        &mut self,
        type_decl: &mut TypeDeclaration,
        _span: SimpleSpan,
    ) {
        if type_decl.hoisted {
            let id = self
                .scope_stack
                .get_variable_and_update_metadata(
                    &type_decl.name.clone(),
                    &mut self.metadata,
                )
                .ok();
            type_decl.id = id;
        } else {
            type_decl.id =
                Some(self.add_new_variable(
                    type_decl.name.clone(),
                    VariableShape::Type,
                ));
        }
        type_decl.visit_children_with(self);
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
        let mut ast = parse("var x: integer = 34; x").unwrap();
        precompiler.precompile(&mut ast);
    }
}
