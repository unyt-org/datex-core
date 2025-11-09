use crate::stdlib::{cell::RefCell, collections::HashSet, ops::Range, rc::Rc};
use core::str::FromStr;
use core::unreachable;

pub mod options;
pub mod precompiled_ast;
pub mod scope;
pub mod scope_stack;
use crate::ast::structs::ResolvedVariable;
use crate::ast::structs::expression::{
    DatexExpression, RemoteExecution, VariantAccess,
};
use crate::ast::structs::r#type::{
    TypeExpression, TypeExpressionData, TypeVariantAccess,
};
use crate::visitor::type_expression::visitable::TypeExpressionVisitResult;
use crate::{
    ast::{
        parse_result::ValidDatexParseResult,
        spanned::Spanned,
        structs::expression::{
            BinaryOperation, DatexExpressionData, Statements, TypeDeclaration,
            VariableAccess, VariableAssignment, VariableDeclaration,
            VariableKind,
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
    global::operators::{BinaryOperator, binary::ArithmeticOperator},
    libs::core::CoreLibPointerId,
    references::type_reference::{NominalTypeDeclaration, TypeReference},
    types::type_container::TypeContainer,
    values::core_values::r#type::Type,
    visitor::{
        VisitAction,
        expression::{ExpressionVisitor, visitable::ExpressionVisitResult},
        type_expression::TypeExpressionVisitor,
    },
};
use options::PrecompilerOptions;
use precompiled_ast::AstMetadata;
use precompiled_ast::RichAst;
use precompiled_ast::VariableShape;
use scope::NewScopeType;
use scope_stack::PrecompilerScopeStack;

pub struct Precompiler<'a> {
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope_stack: &'a mut PrecompilerScopeStack,
    collected_errors: Option<DetailedCompilerErrors>,
    spans: Vec<Range<usize>>, // FIXME make this better
    is_first_level_expression: bool,
}

/// Precompile the AST by resolving variable references and collecting metadata.
/// Exits early on first error encountered, returning a SpannedCompilerError.
pub fn precompile_ast_simple_error(
    ast: ValidDatexParseResult,
    scope_stack: &mut PrecompilerScopeStack,
    ast_metadata: Rc<RefCell<AstMetadata>>,
) -> Result<RichAst, SpannedCompilerError> {
    precompile_ast(
        ast,
        scope_stack,
        ast_metadata,
        PrecompilerOptions {
            detailed_errors: false,
        },
    )
    .map_err(|e| {
        match e {
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(
                error,
            ) => error,
            _ => unreachable!(), // because detailed_errors: false
        }
    })
}

/// Precompile the AST by resolving variable references and collecting metadata.
/// Collects all errors encountered, returning a DetailedCompilerErrorsWithRichAst.
pub fn precompile_ast_detailed_errors(
    ast: ValidDatexParseResult,
    scope_stack: &mut PrecompilerScopeStack,
    ast_metadata: Rc<RefCell<AstMetadata>>,
) -> Result<RichAst, DetailedCompilerErrorsWithRichAst> {
    precompile_ast(
        ast,
        scope_stack,
        ast_metadata,
        PrecompilerOptions {
            detailed_errors: true,
        },
    )
    .map_err(|e| {
        match e {
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Detailed(
                error,
            ) => error,
            _ => unreachable!(), // because detailed_errors: true
        }
    })
}

/// Precompile the AST by resolving variable references and collecting metadata.
pub fn precompile_ast(
    ast: ValidDatexParseResult,
    scope_stack: &mut PrecompilerScopeStack,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    options: PrecompilerOptions,
) -> Result<RichAst, SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst> {
    Precompiler::new(scope_stack, ast_metadata).precompile(ast, options)
}

impl<'a> Precompiler<'a> {
    pub fn new(
        scope_stack: &'a mut PrecompilerScopeStack,
        ast_metadata: Rc<RefCell<AstMetadata>>,
    ) -> Self {
        Self {
            ast_metadata,
            scope_stack,
            collected_errors: None,
            spans: vec![],
            is_first_level_expression: true,
        }
    }

    /// Collects an error if detailed error collection is enabled,
    /// or returns the error as Err()
    fn collect_error(
        &mut self,
        error: SpannedCompilerError,
    ) -> Result<(), SpannedCompilerError> {
        match &mut self.collected_errors {
            Some(collected_errors) => {
                collected_errors.record_error(error);
                Ok(())
            }
            None => Err(error),
        }
    }

    /// Collects the Err variant of the Result if detailed error collection is enabled,
    /// or returns the Result mapped to a MaybeAction.
    fn collect_result<T>(
        &mut self,
        result: Result<T, SpannedCompilerError>,
    ) -> Result<MaybeAction<T>, SpannedCompilerError> {
        collect_or_pass_error(&mut self.collected_errors, result)
    }

    fn get_variable_and_update_metadata(
        &mut self,
        name: &str,
    ) -> Result<usize, CompilerError> {
        self.scope_stack.get_variable_and_update_metadata(
            name,
            &mut self.ast_metadata.borrow_mut(),
        )
    }

    /// Precompile the AST by resolving variable references and collecting metadata.
    fn precompile(
        mut self,
        mut ast: ValidDatexParseResult,
        options: PrecompilerOptions,
    ) -> Result<RichAst, SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst>
    {
        if options.detailed_errors {
            self.collected_errors = Some(DetailedCompilerErrors::default());
        }
        self.spans = ast.spans.clone(); // FIXME make better

        // visit ast recursively
        // returns Error directly if early exit on first error is enabled
        self.visit_datex_expression(&mut ast.ast)?;

        let mut rich_ast = RichAst {
            metadata: self.ast_metadata,
            ast: Some(ast.ast),
        };

        // type inference - currently only if detailed errors are enabled
        // FIXME: always do type inference here, not only for detailed errors
        if options.detailed_errors {
            let type_res = infer_expression_type_detailed_errors(
                rich_ast.ast.as_mut().unwrap(),
                rich_ast.metadata.clone(),
            );

            // append type errors to collected_errors if any
            if let Some(collected_errors) = self.collected_errors.as_mut()
                && let Err(type_errors) = type_res
            {
                collected_errors.append(type_errors.into());
            }
        }

        // if collecting detailed errors and an error occurred, return
        if let Some(errors) = self.collected_errors
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
        let new_id = self.ast_metadata.borrow().variables.len();
        let var_metadata =
            self.scope_stack
                .add_new_variable(name.clone(), new_id, kind);
        self.ast_metadata.borrow_mut().variables.push(var_metadata);
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
        if let Ok(id) = self.get_variable_and_update_metadata(name) {
            Ok(ResolvedVariable::VariableId(id))
        }
        // try to resolve core variable
        else if let Ok(core) = CoreLibPointerId::from_str(name) {
            Ok(ResolvedVariable::PointerAddress(core.into()))
        } else {
            Err(CompilerError::UndeclaredVariable(name.to_string()))
        }
    }

    fn scope_type_for_expression(
        &mut self,
        expr: &DatexExpression,
    ) -> NewScopeType {
        match &expr.data {
            DatexExpressionData::RemoteExecution(_) => NewScopeType::None,
            _ => NewScopeType::NewScope,
        }
    }
}

impl<'a> TypeExpressionVisitor<SpannedCompilerError> for Precompiler<'a> {
    fn before_visit_type_expression(&mut self, expr: &mut TypeExpression) {
        if let Some(new_span) = self.span(&expr.span) {
            expr.span = new_span;
        }
    }

    fn visit_literal_type(
        &mut self,
        literal: &mut String,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedCompilerError> {
        let resolved_variable = self.resolve_variable(literal)?;
        Ok(VisitAction::Replace(match resolved_variable {
            ResolvedVariable::VariableId(id) => {
                TypeExpressionData::VariableAccess(VariableAccess {
                    id,
                    name: literal.to_string(),
                })
                .with_span(span.clone())
            }
            ResolvedVariable::PointerAddress(pointer_address) => {
                TypeExpressionData::GetReference(pointer_address)
                    .with_span(span.clone())
            }
        }))
    }
    fn visit_variant_access_type(
        &mut self,
        variant_access: &mut TypeVariantAccess,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedCompilerError> {
        // ensure lhs exist
        let _ = self.resolve_variable(&variant_access.name)?;
        let literal =
            format!("{}/{}", variant_access.name, variant_access.variant);

        // resolve full variant access
        let resolved_variable = self.resolve_variable(&literal)?;
        Ok(VisitAction::Replace(match resolved_variable {
            ResolvedVariable::VariableId(id) => {
                TypeExpressionData::VariableAccess(VariableAccess {
                    id,
                    name: literal,
                })
                .with_span(span.clone())
            }
            ResolvedVariable::PointerAddress(pointer_address) => {
                TypeExpressionData::GetReference(pointer_address)
                    .with_span(span.clone())
            }
        }))
    }
}
impl<'a> ExpressionVisitor<SpannedCompilerError> for Precompiler<'a> {
    /// Handle expression errors by either recording them if collected_errors is Some,
    /// or aborting the traversal if collected_errors is None.
    fn handle_expression_error(
        &mut self,
        error: SpannedCompilerError,
        _expression: &DatexExpression,
    ) -> Result<VisitAction<DatexExpression>, SpannedCompilerError> {
        if let Some(collected_errors) = self.collected_errors.as_mut() {
            collected_errors.record_error(error);
            Ok(VisitAction::VisitChildren)
        } else {
            Err(error)
        }
    }

    fn before_visit_datex_expression(&mut self, expr: &mut DatexExpression) {
        if let Some(new_span) = self.span(&expr.span) {
            expr.span = new_span;
        }

        match self.scope_type_for_expression(expr) {
            NewScopeType::NewScopeWithNewRealm => {
                self.scope_stack.push_scope();
                self.scope_stack.increment_realm_index();
            }
            NewScopeType::NewScope => {
                // if in top level scope, don't create a new scope if first ast level
                if !(self.scope_stack.scopes.len() == 1
                    && self.is_first_level_expression)
                {
                    self.scope_stack.push_scope();
                }
            }
            _ => {}
        };

        self.is_first_level_expression = false;
    }

    fn after_visit_datex_expression(&mut self, expr: &mut DatexExpression) {
        match self.scope_type_for_expression(expr) {
            NewScopeType::NewScope | NewScopeType::NewScopeWithNewRealm => {
                // always keep top level scope
                if self.scope_stack.scopes.len() > 1 {
                    self.scope_stack.pop_scope();
                }
            }
            _ => {}
        };
    }

    fn visit_remote_execution(
        &mut self,
        remote_execution: &mut RemoteExecution,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedCompilerError> {
        self.visit_datex_expression(&mut remote_execution.left)?;

        self.scope_stack.push_scope();
        self.scope_stack.increment_realm_index();

        self.visit_datex_expression(&mut remote_execution.right)?;
        self.scope_stack.pop_scope();
        Ok(VisitAction::SkipChildren)
    }

    fn visit_statements(
        &mut self,
        statements: &mut Statements,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedCompilerError> {
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
                    self.collect_error(SpannedCompilerError::new_with_span(
                        CompilerError::InvalidRedeclaration(name.clone()),
                        statements.span.clone(),
                    ))?;
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
                    self.ast_metadata
                        .borrow_mut()
                        .variable_metadata_mut(type_id)
                        .expect("TypeDeclaration should have variable metadata")
                        .var_type = Some(type_def.clone());
                }
            }
        }
        Ok(VisitAction::VisitChildren)
    }

    fn visit_type_declaration(
        &mut self,
        type_declaration: &mut TypeDeclaration,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedCompilerError> {
        let name = type_declaration.name.clone();
        if type_declaration.hoisted {
            let id = self
                .get_variable_and_update_metadata(
                    &type_declaration.name.clone(),
                )
                .ok();
            type_declaration.id = id;
        } else {
            type_declaration.id =
                Some(self.add_new_variable(name, VariableShape::Type));
        }
        Ok(VisitAction::VisitChildren)
    }

    fn visit_binary_operation(
        &mut self,
        binary_operation: &mut BinaryOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedCompilerError> {
        let operator = &binary_operation.operator;

        // handle special case: / operator
        if operator == &BinaryOperator::Arithmetic(ArithmeticOperator::Divide) {
            let left = &mut binary_operation.left;
            let right = &mut binary_operation.right;

            let lit_left =
                if let DatexExpressionData::Identifier(name) = &left.data {
                    name.clone()
                } else {
                    return Ok(VisitAction::VisitChildren);
                };
            let lit_right =
                if let DatexExpressionData::Identifier(name) = &right.data {
                    name.clone()
                } else {
                    return Ok(VisitAction::VisitChildren);
                };
            // both of the sides are identifiers
            let left_var = self.resolve_variable(lit_left.as_str());
            let is_right_defined =
                self.resolve_variable(lit_right.as_str()).is_ok();

            // left is defined (could be integer, or user defined variable)
            if let Ok(left_var) = left_var {
                if is_right_defined {
                    // both sides are defined, left side could be a type, or no,
                    // same for right side
                    // could be variant access if the left side is a type and right side does exist as subvariant,
                    // otherwise we try division
                    Ok(VisitAction::VisitChildren)
                } else {
                    // is right is not defined, fallback to variant access
                    // could be divison though, where user misspelled rhs (unhandled, will throw)
                    Ok(VisitAction::Replace(DatexExpression::new(
                        DatexExpressionData::VariantAccess(VariantAccess {
                            base: left_var,
                            name: lit_left,
                            variant: lit_right,
                        }),
                        span.clone(),
                    )))
                }
            } else {
                Ok(VisitAction::VisitChildren)
            }
        } else {
            Ok(VisitAction::VisitChildren)
        }
    }

    fn visit_variable_declaration(
        &mut self,
        variable_declaration: &mut VariableDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedCompilerError> {
        // check if variable already declared in active scope
        if let Some(existing_var_id) = self
            .scope_stack
            .get_active_scope()
            .variable_ids_by_name
            .get(&variable_declaration.name)
        {
            variable_declaration.id = Some(*existing_var_id);
            return Err(SpannedCompilerError::new_with_span(
                CompilerError::InvalidRedeclaration(
                    variable_declaration.name.clone(),
                ),
                span.clone(),
            ));
        }
        variable_declaration.id = Some(self.add_new_variable(
            variable_declaration.name.clone(),
            VariableShape::Value(variable_declaration.kind),
        ));
        Ok(VisitAction::VisitChildren)
    }

    fn visit_variable_assignment(
        &mut self,
        variable_assignment: &mut VariableAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedCompilerError> {
        let res = self
            .get_variable_and_update_metadata(&variable_assignment.name)
            .map_err(|error| {
                SpannedCompilerError::new_with_span(error, span.clone())
            });
        let action = self.collect_result(res)?;
        if let MaybeAction::Do(new_id) = action {
            // continue
            // check if variable is const
            let var_shape = self
                .ast_metadata
                .borrow()
                .variable_metadata(new_id)
                .unwrap()
                .shape;
            variable_assignment.id = Some(new_id);
            if let VariableShape::Value(VariableKind::Const) = var_shape {
                self.collect_error(SpannedCompilerError::new_with_span(
                    CompilerError::AssignmentToConst(
                        variable_assignment.name.clone(),
                    ),
                    span.clone(),
                ))?;
            };
        }
        Ok(VisitAction::VisitChildren)
    }

    fn visit_identifier(
        &mut self,
        identifier: &mut String,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedCompilerError> {
        let result = self.resolve_variable(identifier).map_err(|error| {
            SpannedCompilerError::new_with_span(error, span.clone())
        });
        let action = self.collect_result(result)?;
        if let MaybeAction::Do(resolved_variable) = action {
            return Ok(VisitAction::Replace(match resolved_variable {
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
            }));
        }
        Ok(VisitAction::SkipChildren)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::error::src::SrcId;
    use crate::ast::parse;
    use crate::ast::parse_result::{DatexParseResult, InvalidDatexParseResult};
    use crate::ast::structs::expression::{CreateRef, Deref};
    use crate::ast::structs::r#type::{StructuralMap, TypeExpressionData};
    use crate::references::reference::ReferenceMutability;
    use crate::stdlib::assert_matches::assert_matches;
    use crate::stdlib::io;
    use crate::values::core_values::integer::Integer;
    use crate::values::pointer::PointerAddress;

    fn precompile(
        ast: ValidDatexParseResult,
        options: PrecompilerOptions,
    ) -> Result<RichAst, SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst>
    {
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        Precompiler::new(&mut scope_stack, ast_metadata)
            .precompile(ast, options)
    }

    #[test]
    fn test_precompiler_visit() {
        let options = PrecompilerOptions::default();
        let ast = parse("var x: integer = 34; var y = 10; x + y").unwrap();
        let res = precompile(ast, options).unwrap();
        println!("{:#?}", res.ast);
    }

    #[test]
    fn undeclared_variable_error() {
        let options = PrecompilerOptions {
            detailed_errors: true,
        };
        let ast = parse("x + 10").unwrap();
        let result = precompile(ast, options);
        println!("{:#?}", result);
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_variable_error() {
        let options = PrecompilerOptions {
            detailed_errors: false,
        };
        let ast = parse("var x = 1; var x = 2;").unwrap();
        let result = precompile(ast, options);
        assert_matches!(result.unwrap_err(), SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(SpannedCompilerError{span, error: CompilerError::InvalidRedeclaration(name)})  if name == "x");
    }

    #[test]
    fn invalid_type_redeclaration() {
        let src = r#"
        type A = integer;
        type A = text; // redeclaration error
        "#;
        let ast = parse(src).unwrap();
        let result = precompile(ast, PrecompilerOptions::default());
        assert!(result.is_err());
        assert_matches!(
            result,
            Err(SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(SpannedCompilerError{span, error: CompilerError::InvalidRedeclaration(name)})) if name == "A"
        );
    }

    fn parse_unwrap(src: &str) -> DatexExpression {
        let src_id = SrcId::test();
        let res = parse(src);
        if let DatexParseResult::Invalid(InvalidDatexParseResult {
            errors,
            ..
        }) = res
        {
            errors.iter().for_each(|e| {
                let cache = ariadne::sources(vec![(src_id, src)]);
                e.clone().write(cache, io::stdout());
            });
            core::panic!("Parsing errors found");
        }
        res.unwrap().ast
    }

    fn parse_and_precompile_spanned_result(
        src: &str,
    ) -> Result<RichAst, SpannedCompilerError> {
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let ast = parse(src)
            .to_result()
            .map_err(|mut e| SpannedCompilerError::from(e.remove(0)))?;
        precompile_ast_simple_error(ast, &mut scope_stack, ast_metadata)
    }

    fn parse_and_precompile(src: &str) -> Result<RichAst, CompilerError> {
        parse_and_precompile_spanned_result(src).map_err(|e| e.error)
    }

    #[test]
    fn undeclared_variable() {
        let result = parse_and_precompile_spanned_result("x + 42");
        assert!(result.is_err());
        assert_matches!(
            result,
            Err(SpannedCompilerError{ error: CompilerError::UndeclaredVariable(var_name), span })
            if var_name == "x" && span == Some((0..1))
        );
    }

    #[test]
    fn scoped_variable() {
        let result = parse_and_precompile("(var z = 42;z); z");
        assert!(result.is_err());
        assert_matches!(
            result,
            Err(CompilerError::UndeclaredVariable(var_name))
            if var_name == "z"
        );
    }

    #[test]
    fn core_types() {
        let result = parse_and_precompile("boolean");
        assert_matches!(
            result,
            Ok(
                RichAst {
                    ast: Some(DatexExpression { data: DatexExpressionData::GetReference(pointer_id), ..}),
                    ..
                }
            ) if pointer_id == CoreLibPointerId::Boolean.into()
        );
        let result = parse_and_precompile("integer");
        assert_matches!(
            result,
            Ok(
                RichAst {
                    ast: Some(DatexExpression { data: DatexExpressionData::GetReference(pointer_id), ..}),
                    ..
                }
            ) if pointer_id == CoreLibPointerId::Integer(None).into()
        );

        let result = parse_and_precompile("integer/u8");
        assert_eq!(
            result.unwrap().ast,
            Some(
                DatexExpressionData::VariantAccess(VariantAccess {
                    base: ResolvedVariable::PointerAddress(
                        CoreLibPointerId::Integer(None).into()
                    ),
                    name: "integer".to_string(),
                    variant: "u8".to_string(),
                })
                .with_default_span()
            )
        );
    }

    #[test]
    fn variant_access() {
        // core type should work
        let result =
            parse_and_precompile("integer/u8").expect("Precompilation failed");
        assert_eq!(
            result.ast,
            Some(
                DatexExpressionData::VariantAccess(VariantAccess {
                    base: ResolvedVariable::PointerAddress(
                        CoreLibPointerId::Integer(None).into()
                    ),
                    name: "integer".to_string(),
                    variant: "u8".to_string(),
                })
                .with_default_span()
            )
        );

        // invalid variant should work (will error later in type checking)
        let result = parse_and_precompile("integer/invalid").unwrap();
        assert_eq!(
            result.ast,
            Some(
                DatexExpressionData::VariantAccess(VariantAccess {
                    base: ResolvedVariable::PointerAddress(
                        CoreLibPointerId::Integer(None).into()
                    ),
                    name: "integer".to_string(),
                    variant: "invalid".to_string(),
                })
                .with_default_span()
            )
        );

        // unknown type should error
        let result = parse_and_precompile("invalid/u8");
        assert_matches!(result, Err(CompilerError::UndeclaredVariable(var_name)) if var_name == "invalid");

        // a variant access without declaring the super type should error
        let result = parse_and_precompile("type User/admin = {}; User/admin");
        assert!(result.is_err());
        assert_matches!(result, Err(CompilerError::UndeclaredVariable(var_name)) if var_name == "User");

        // declared subtype should work
        let result = parse_and_precompile(
            "type User = {}; type User/admin = {}; User/admin",
        );
        assert!(result.is_ok());
        let rich_ast = result.unwrap();
        assert_eq!(
            rich_ast.ast,
            Some(
                DatexExpressionData::Statements(Statements::new_unterminated(
                    vec![
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(0),
                            name: "User".to_string(),
                            value: TypeExpressionData::StructuralMap(
                                StructuralMap(vec![])
                            )
                            .with_default_span(),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(1),
                            name: "User/admin".to_string(),
                            value: TypeExpressionData::StructuralMap(
                                StructuralMap(vec![])
                            )
                            .with_default_span(),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::VariantAccess(VariantAccess {
                            base: ResolvedVariable::VariableId(0),
                            name: "User".to_string(),
                            variant: "admin".to_string(),
                        })
                        .with_default_span()
                    ]
                ))
                .with_default_span()
            )
        );

        // value shall be interpreted as division
        let result = parse_and_precompile("var a = 42; var b = 69; a/b");
        assert!(result.is_ok());
        let statements = if let DatexExpressionData::Statements(stmts) =
            result.unwrap().ast.unwrap().data
        {
            stmts
        } else {
            core::panic!("Expected statements");
        };
        assert_eq!(
            *statements.statements.get(2).unwrap(),
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::VariableAccess(VariableAccess {
                        id: 0,
                        name: "a".to_string()
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::VariableAccess(VariableAccess {
                        id: 1,
                        name: "b".to_string()
                    })
                    .with_default_span()
                ),
                r#type: None
            })
            .with_default_span()
        );

        // type with value should be interpreted as division
        let result = parse_and_precompile("var a = 10; type b = 42; a/b");
        assert!(result.is_ok());
        let statements = if let DatexExpressionData::Statements(stmts) =
            result.unwrap().ast.unwrap().data
        {
            stmts
        } else {
            core::panic!("Expected statements");
        };
        assert_eq!(
            *statements.statements.get(2).unwrap(),
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::VariableAccess(VariableAccess {
                        id: 1,
                        name: "a".to_string()
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::VariableAccess(VariableAccess {
                        id: 0,
                        name: "b".to_string()
                    })
                    .with_default_span()
                ),
                r#type: None
            })
            .with_default_span()
        );
    }

    #[test]
    fn test_type_declaration_assigment() {
        let result = parse_and_precompile("type MyInt = 1; var x = MyInt;");
        assert!(result.is_ok());
        let rich_ast = result.unwrap();
        assert_eq!(
            rich_ast.ast,
            Some(
                DatexExpressionData::Statements(Statements::new_terminated(
                    vec![
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(0),
                            name: "MyInt".to_string(),
                            value: TypeExpressionData::Integer(Integer::from(
                                1
                            ))
                            .with_default_span(),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::VariableDeclaration(
                            VariableDeclaration {
                                id: Some(1),
                                kind: VariableKind::Var,
                                name: "x".to_string(),
                                // must refer to variable id 0
                                init_expression: Box::new(
                                    DatexExpressionData::VariableAccess(
                                        VariableAccess {
                                            id: 0,
                                            name: "MyInt".to_string()
                                        }
                                    )
                                    .with_default_span()
                                ),
                                type_annotation: None,
                            }
                        )
                        .with_default_span(),
                    ]
                ))
                .with_default_span()
            )
        )
    }

    #[test]
    fn test_type_declaration_hoisted_assigment() {
        let result = parse_and_precompile("var x = MyInt; type MyInt = 1;");
        assert!(result.is_ok());
        let rich_ast = result.unwrap();
        assert_eq!(
            rich_ast.ast,
            Some(
                DatexExpressionData::Statements(Statements::new_terminated(
                    vec![
                        DatexExpressionData::VariableDeclaration(
                            VariableDeclaration {
                                id: Some(1),
                                kind: VariableKind::Var,
                                name: "x".to_string(),
                                // must refer to variable id 0
                                init_expression: Box::new(
                                    DatexExpressionData::VariableAccess(
                                        VariableAccess {
                                            id: 0,
                                            name: "MyInt".to_string()
                                        }
                                    )
                                    .with_default_span()
                                ),
                                type_annotation: None,
                            }
                        )
                        .with_default_span(),
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(0),
                            name: "MyInt".to_string(),
                            value: TypeExpressionData::Integer(Integer::from(
                                1
                            ))
                            .with_default_span(),
                            hoisted: true,
                        })
                        .with_default_span(),
                    ]
                ))
                .with_default_span()
            )
        )
    }

    #[test]
    fn test_type_declaration_hoisted_cross_assigment() {
        let result = parse_and_precompile("type x = MyInt; type MyInt = x;");
        assert!(result.is_ok());
        let rich_ast = result.unwrap();
        assert_eq!(
            rich_ast.ast,
            Some(
                DatexExpressionData::Statements(Statements::new_terminated(
                    vec![
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(0),
                            name: "x".to_string(),
                            value: TypeExpressionData::VariableAccess(
                                VariableAccess {
                                    id: 1,
                                    name: "MyInt".to_string()
                                }
                            )
                            .with_default_span(),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(1),
                            name: "MyInt".to_string(),
                            value: TypeExpressionData::VariableAccess(
                                VariableAccess {
                                    id: 0,
                                    name: "x".to_string()
                                }
                            )
                            .with_default_span(),
                            hoisted: true,
                        })
                        .with_default_span(),
                    ]
                ))
                .with_default_span()
            )
        )
    }

    #[test]
    fn test_type_invalid_nested_type_declaration() {
        let result = parse_and_precompile(
            "type x = NestedVar; (1; type NestedVar = x;)",
        );
        assert_matches!(result, Err(CompilerError::UndeclaredVariable(var_name)) if var_name == "NestedVar");
    }

    #[test]
    fn test_type_valid_nested_type_declaration() {
        let result =
            parse_and_precompile("type x = 10; (1; type NestedVar = x;)");
        assert!(result.is_ok());
        let rich_ast = result.unwrap();
        assert_eq!(
            rich_ast.ast,
            Some(
                DatexExpressionData::Statements(Statements::new_unterminated(
                    vec![
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(0),
                            name: "x".to_string(),
                            value: TypeExpressionData::Integer(
                                Integer::from(10).into()
                            )
                            .with_default_span(),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::Statements(
                            Statements::new_terminated(vec![
                                DatexExpressionData::Integer(Integer::from(1))
                                    .with_default_span(),
                                DatexExpressionData::TypeDeclaration(
                                    TypeDeclaration {
                                        id: Some(1),
                                        name: "NestedVar".to_string(),
                                        value:
                                            TypeExpressionData::VariableAccess(
                                                VariableAccess {
                                                    id: 0,
                                                    name: "x".to_string()
                                                }
                                            )
                                            .with_default_span(),
                                        hoisted: true,
                                    }
                                )
                                .with_default_span(),
                            ])
                        )
                        .with_default_span()
                    ]
                ))
                .with_default_span()
            )
        )
    }

    #[test]
    fn test_core_reference_type() {
        let result = parse_and_precompile("type x = integer");
        assert!(result.is_ok());
        let rich_ast = result.unwrap();
        assert_eq!(
            rich_ast.ast,
            Some(
                DatexExpressionData::TypeDeclaration(TypeDeclaration {
                    id: Some(0),
                    name: "x".to_string(),
                    value: TypeExpressionData::GetReference(
                        PointerAddress::from(CoreLibPointerId::Integer(None))
                    )
                    .with_default_span(),
                    hoisted: false,
                })
                .with_default_span()
            )
        );
    }

    #[test]
    fn test_deref() {
        let result = parse_and_precompile("const x = &42; *x");
        assert!(result.is_ok());
        let rich_ast = result.unwrap();
        assert_eq!(
            rich_ast.ast,
            Some(
                DatexExpressionData::Statements(Statements::new_unterminated(
                    vec![
                        DatexExpressionData::VariableDeclaration(
                            VariableDeclaration {
                                id: Some(0),
                                kind: VariableKind::Const,
                                name: "x".to_string(),
                                init_expression: Box::new(
                                    DatexExpressionData::CreateRef(CreateRef {
                                        mutability:
                                            ReferenceMutability::Immutable,
                                        expression: Box::new(
                                            DatexExpressionData::Integer(
                                                Integer::from(42)
                                            )
                                            .with_default_span()
                                        )
                                    })
                                    .with_default_span(),
                                ),
                                type_annotation: None,
                            }
                        )
                        .with_default_span(),
                        DatexExpressionData::Deref(Deref {
                            expression: Box::new(
                                DatexExpressionData::VariableAccess(
                                    VariableAccess {
                                        id: 0,
                                        name: "x".to_string()
                                    }
                                )
                                .with_default_span()
                            )
                        })
                        .with_default_span(),
                    ]
                ))
                .with_default_span()
            )
        );
    }
}
