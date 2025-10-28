use std::{cell::RefCell, ops::Range, rc::Rc};

use log::info;

use crate::{
    ast::{
        data::{
            expression::{
                DatexExpression, DatexExpressionData, Statements,
                TypeDeclaration, VariableAccess, VariableAssignment,
                VariableDeclaration, VariableKind,
            },
            spanned::Spanned,
            r#type::TypeExpression,
            visitor::{VisitMut, Visitable},
        },
        parse_result::ValidDatexParseResult,
    },
    compiler::{
        error::{
            CompilerError, DetailedCompilerErrors,
            DetailedCompilerErrorsWithRichAst, ErrorCollector, MaybeAction,
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst,
            SpannedCompilerError, collect_or_pass_error,
        },
        precompiler::{
            AstMetadata, PrecompilerOptions, PrecompilerScopeStack, RichAst,
            VariableShape,
        },
        type_inference::infer_expression_type_detailed_errors,
    },
    libs::core::CoreLibPointerId,
    references::type_reference::{NominalTypeDeclaration, TypeReference},
    types::type_container::TypeContainer,
    values::{
        core_values::r#type::Type, pointer::PointerAddress,
        value_container::ValueContainer,
    },
};

pub struct Precompiler {
    options: PrecompilerOptions,
    spans: Vec<Range<usize>>,
    metadata: Option<AstMetadata>,
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
            metadata: None,
            scope_stack: PrecompilerScopeStack::default(),
            errors: None,
        }
    }

    fn metadata(&self) -> &AstMetadata {
        self.metadata
            .as_ref()
            .expect("Metadata must be initialized")
    }
    fn metadata_mut(&mut self) -> &mut AstMetadata {
        self.metadata
            .as_mut()
            .expect("Metadata must be initialized")
    }

    /// Precompile the AST by resolving variable references and collecting metadata.
    pub fn precompile(
        &mut self,
        ast: &mut ValidDatexParseResult,
    ) -> Result<RichAst, SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst>
    {
        self.metadata = Some(AstMetadata::default());
        self.scope_stack = PrecompilerScopeStack::default();
        self.spans = ast.spans.clone();

        self.errors = if self.options.detailed_errors {
            Some(DetailedCompilerErrors::default())
        } else {
            None
        };

        self.visit_expression(&mut ast.ast);

        let mut rich_ast = RichAst {
            metadata: Rc::new(RefCell::new(self.metadata.take().unwrap())),
            ast: Some(ast.ast.clone()), // FIXME store as ref and avoid clone
        };

        // type inference - currently only if detailed errors are enabled
        // FIXME: always do type inference here, not only for detailed errors
        if self.options.detailed_errors {
            let type_res = infer_expression_type_detailed_errors(
                rich_ast.ast.as_mut().unwrap(),
                rich_ast.metadata.clone(),
            );

            // append type errors to collected_errors if any
            if let Some(collected_errors) = self.errors.as_mut()
                && let Err(type_errors) = type_res
            {
                collected_errors.append(type_errors.into());
            }
        }

        // if collecting detailed errors and an error occurred, return
        if let Some(errors) = self.errors.take()
            && errors.has_errors()
        {
            Err(
                SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Detailed(
                    DetailedCompilerErrorsWithRichAst {
                        errors,
                        ast: rich_ast,
                    },
                ),
            )
        } else {
            Ok(rich_ast)
        }
    }

    /// Get the full span from start and end token indices
    /// Returns None if the span is the default (0..0)
    /// Used to convert token indices to actual spans in the source code
    fn span(&self, span: &Range<usize>) -> Option<Range<usize>> {
        // skip if both zero (default span used for testing)
        // TODO: improve this
        if span.start != 0 || span.end != 0 {
            let start_token = self.spans.get(span.start).cloned().unwrap();
            let end_token = self.spans.get(span.end - 1).cloned().unwrap();
            Some(start_token.start..end_token.end)
        } else {
            None
        }
    }

    /// Adds a new variable to the current scope and metadata
    /// Returns the new variable ID
    fn add_new_variable(&mut self, name: String, kind: VariableShape) -> usize {
        let new_id = self.metadata_mut().variables.len();
        let var_metadata =
            self.scope_stack
                .add_new_variable(name.clone(), new_id, kind);
        self.metadata_mut().variables.push(var_metadata);
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
        if let Ok(id) = self.scope_stack.get_variable_and_update_metadata(
            name,
            self.metadata.as_mut().unwrap(),
        ) {
            info!("Visiting variable: {name}");
            Ok(ResolvedVariable::VariableId(id))
        }
        // try to resolve core variable
        else if let Some(core) = self.metadata()
        .runtime
        .memory()
        .borrow()
        .get_reference(&CoreLibPointerId::Core.into()) // FIXME don't use core struct here, but better access with one of our mappings already present
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
