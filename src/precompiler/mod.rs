use std::{cell::RefCell, collections::HashSet, ops::Range, rc::Rc};

use log::info;
pub mod options;
pub mod precompiled_ast;
pub mod scope;
pub mod scope_stack;
use crate::{
    ast::{
        parse_result::ValidDatexParseResult,
        spanned::Spanned,
        structs::{
            expression::{
                BinaryOperation, DatexExpressionData, Statements,
                TypeDeclaration, VariableAccess, VariableAssignment,
                VariableDeclaration, VariableKind,
            },
            operator::{BinaryOperator, binary::ArithmeticOperator},
        },
    },
    compiler::{
        error::{
            CompilerError, DetailedCompilerErrors,
            DetailedCompilerErrorsWithRichAst, ErrorCollector, MaybeAction,
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst,
            SpannedCompilerError, collect_or_pass_error,
        },
        type_inference::infer_expression_type_detailed_errors,
    },
    libs::core::CoreLibPointerId,
    precompiler::{
        options::PrecompilerOptions,
        precompiled_ast::{AstMetadata, RichAst, VariableShape},
        scope_stack::PrecompilerScopeStack,
    },
    references::type_reference::{NominalTypeDeclaration, TypeReference},
    types::type_container::TypeContainer,
    values::{
        core_values::r#type::Type, pointer::PointerAddress,
        value_container::ValueContainer,
    },
    visitor::{
        VisitAction,
        expression::{ExpressionVisitor, visitable::ExpressionVisitAction},
        type_expression::TypeExpressionVisitor,
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

        self.visit_datex_expression(&mut ast.ast);

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

impl TypeExpressionVisitor for Precompiler {}
impl ExpressionVisitor for Precompiler {
    fn visit_variable_declaration(
        &mut self,
        variable_declaration: &mut VariableDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitAction {
        variable_declaration.id = Some(self.add_new_variable(
            variable_declaration.name.clone(),
            VariableShape::Value(variable_declaration.kind),
        ));
        VisitAction::VisitChildren
    }

    fn visit_type_declaration(
        &mut self,
        type_declaration: &mut TypeDeclaration,
        _: &Range<usize>,
    ) -> ExpressionVisitAction {
        let name = type_declaration.name.clone();
        if type_declaration.hoisted {
            let id = self
                .scope_stack
                .get_variable_and_update_metadata(
                    &type_declaration.name.clone(),
                    self.metadata.as_mut().unwrap(),
                )
                .ok();
            type_declaration.id = id;
        } else {
            type_declaration.id =
                Some(self.add_new_variable(name, VariableShape::Type));
        }
        VisitAction::VisitChildren
    }

    fn visit_variable_assignment(
        &mut self,
        variable_assignment: &mut VariableAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitAction {
        let new_id = self
            .scope_stack
            .get_variable_and_update_metadata(
                &variable_assignment.name,
                self.metadata.as_mut().unwrap(),
            )
            .unwrap(); // FIXME: handle error properly
        // check if variable is const
        let var_metadata = self
            .metadata()
            .variable_metadata(new_id)
            .expect("Variable must have metadata");
        if let VariableShape::Value(VariableKind::Const) = var_metadata.shape {
            let error = SpannedCompilerError::new_with_span(
                CompilerError::AssignmentToConst(
                    variable_assignment.name.clone(),
                ),
                span.clone(),
            );
            match &mut self.errors {
                Some(collected_errors) => {
                    collected_errors.record_error(error);
                }
                None => return VisitAction::ToNoop, // FIXME return error
            }
        }
        variable_assignment.id = Some(new_id);
        VisitAction::VisitChildren
    }

    fn visit_statements(
        &mut self,
        statements: &mut Statements,
        _: &Range<usize>,
    ) -> ExpressionVisitAction {
        let mut registered_names = HashSet::new();
        for statements in statements.statements.iter_mut() {
            if let DatexExpressionData::TypeDeclaration(TypeDeclaration {
                name,
                hoisted,
                ..
            }) = &mut statements.data
            {
                // set hoisted to true
                *hoisted = true;
                if registered_names.contains(name) {
                    let error = SpannedCompilerError::new_with_span(
                        CompilerError::InvalidRedeclaration(name.clone()),
                        statements.span.clone(),
                    );
                    match &mut self.errors {
                        Some(collected_errors) => {
                            collected_errors.record_error(error);
                        }
                        None => return VisitAction::ToNoop, // FIXME return error
                    }
                }
                registered_names.insert(name.clone());

                // register variable
                let type_id =
                    self.add_new_variable(name.clone(), VariableShape::Type);

                // register placeholder ref in metadata
                let reference = Rc::new(RefCell::new(TypeReference::nominal(
                    Type::UNIT,
                    NominalTypeDeclaration::from(name.to_string()),
                    None,
                )));
                let type_def = TypeContainer::TypeReference(reference.clone());
                {
                    self.metadata_mut()
                        .variable_metadata_mut(type_id)
                        .expect("TypeDeclaration should have variable metadata")
                        .var_type = Some(type_def.clone());
                }
            }
        }
        VisitAction::VisitChildren
    }

    fn visit_identifier(
        &mut self,
        identifier: &mut String,
        span: &Range<usize>,
    ) -> ExpressionVisitAction {
        let result = self.resolve_variable(identifier).map_err(|error| {
            SpannedCompilerError::new_with_span(error, span.clone())
        });
        let action = collect_or_pass_error(&mut self.errors, result).unwrap(); // FIXME: handle error properly
        if let MaybeAction::Do(resolved_variable) = action {
            return VisitAction::Replace(match resolved_variable {
                ResolvedVariable::VariableId(id) => {
                    DatexExpressionData::VariableAccess(VariableAccess {
                        id,
                        name: identifier.clone(),
                    })
                    .with_span(span.clone())
                }
                ResolvedVariable::PointerAddress(pointer_address) => {
                    DatexExpressionData::GetReference(pointer_address)
                        .with_span(span.clone())
                }
            });
        }
        VisitAction::SkipChildren
    }

    fn visit_binary_operation(
        &mut self,
        binary_operation: &mut BinaryOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitAction {
        let operator = &binary_operation.operator;
        let left = &mut binary_operation.left;
        let right = &mut binary_operation.right;

        // handle variant access operator
        if matches!(operator, BinaryOperator::VariantAccess) {
            let lit_left = if let DatexExpressionData::Identifier(name) =
                &left.data
            {
                name.clone()
            } else {
                unreachable!("Left side of variant access must be a literal");
            };

            let lit_right = if let DatexExpressionData::Identifier(name) =
                &right.data
            {
                name.clone()
            } else {
                unreachable!("Right side of variant access must be a literal");
            };
            let full_name = format!("{lit_left}/{lit_right}");
            // if get_variable_kind(lhs) == Value
            // 1. user value lhs, whatever rhs -> division

            // if get_variable_kind(lhs) == Type
            // 2. lhs is a user defined type, so
            // lhs/rhs should be also, otherwise
            // this throws VariantNotFound

            // if resolve_variable(lhs)
            // this must be a core type
            // if resolve_variable(lhs/rhs) has
            // and error, this throws VariantNotFound

            // Check if the left literal is a variable (value or type, but no core type)
            if self.scope_stack.has_variable(lit_left.as_str()) {
                match self
                    .scope_stack
                    .variable_kind(lit_left.as_str(), self.metadata())
                    .unwrap()
                {
                    VariableShape::Type => {
                        // user defined type, continue to variant access
                        let resolved_variable = self
                            .resolve_variable(&full_name)
                            .map_err(|_| {
                                CompilerError::SubvariantNotFound(
                                    lit_left.to_string(),
                                    lit_right.to_string(),
                                )
                            })
                            .unwrap(); // FIXME: handle error properly
                        return VisitAction::Replace(match resolved_variable {
                            ResolvedVariable::VariableId(id) => {
                                DatexExpressionData::VariableAccess(
                                    VariableAccess {
                                        id,
                                        name: full_name.to_string(),
                                    },
                                )
                                .with_span(span.clone())
                            }
                            _ => unreachable!(
                                "Variant access must resolve to a core library type"
                            ),
                        });
                    }
                    VariableShape::Value(_) => {
                        // user defined value, this is a division
                        return VisitAction::ReplaceRecurseChildNodes(
                            DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::Arithmetic(
                                        ArithmeticOperator::Divide,
                                    ),
                                    left: left.to_owned(),
                                    right: right.to_owned(),
                                    r#type: None,
                                },
                            )
                            .with_span(span.clone()),
                        );
                    }
                }
            }
            // can be either a core type or a undeclared variable

            // check if left part is a core value / type
            // otherwise throw the error
            self.resolve_variable(lit_left.as_str()).unwrap(); // FIXME: handle error properly

            let resolved_variable = self
                .resolve_variable(format!("{lit_left}/{lit_right}").as_str())
                .map_err(|error| {
                    SpannedCompilerError::new_with_span(
                        CompilerError::SubvariantNotFound(lit_left, lit_right),
                        span.clone(),
                    )
                });
            let action =
                collect_or_pass_error(&mut self.errors, resolved_variable)
                    .unwrap(); // FIXME: handle error properly
            if let MaybeAction::Do(resolved_variable) = action {
                VisitAction::ReplaceRecurseChildNodes(match resolved_variable {
                    ResolvedVariable::PointerAddress(pointer_address) => {
                        DatexExpressionData::GetReference(pointer_address)
                            .with_span(span.clone())
                    }
                    // FIXME is variable User/whatever allowed here, or
                    // will this always be a reference to the type?
                    _ => unreachable!(
                        "Variant access must resolve to a core library type"
                    ),
                })
            } else {
                unreachable!("Error must have been handled above");
            }
        } else {
            // continue normal processing
            VisitAction::VisitChildren
        }
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
        let mut ast = parse("var x: integer = 34; var y = 10; x + y").unwrap();
        let _ = precompiler.precompile(&mut ast);
        println!("{:#?}", ast);
    }
}
