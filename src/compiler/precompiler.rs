use crate::ast::binary_operation::{ArithmeticOperator, BinaryOperator};
use crate::ast::chain::ApplyOperation;
use crate::ast::tree::{
    ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
    DatexExpression, DatexExpressionData, DerefAssignment, FunctionDeclaration,
    RemoteExecution, SlotAssignment, TypeDeclaration, TypeExpressionData,
    UnaryOperation, VariableAssignment, VariableDeclaration, VariableKind,
};
use crate::compiler::error::{
    CompilerError, DetailedCompilerErrors, ErrorCollector, MaybeAction,
    SpannedCompilerError, collect_or_pass_error,
};
use crate::compiler::error::{
    DetailedCompilerErrorsWithRichAst,
    SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst,
};
use crate::compiler::type_inference::infer_expression_type_detailed_errors;
use crate::libs::core::CoreLibPointerId;
use crate::references::type_reference::{
    NominalTypeDeclaration, TypeReference,
};
use crate::runtime::Runtime;
use crate::types::type_container::TypeContainer;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use chumsky::prelude::SimpleSpan;
use datex_core::ast::parse_result::ValidDatexParseResult;
use datex_core::ast::tree::VariableAccess;
use log::info;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::ops::Range;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct VariableMetadata {
    original_realm_index: usize,
    pub is_cross_realm: bool,
    pub shape: VariableShape,
    pub var_type: Option<TypeContainer>,
    pub name: String,
}

#[derive(Default, Debug)]
pub struct AstMetadata {
    pub variables: Vec<VariableMetadata>,
    // TODO #441: move runtime somewhere else, not in AstMetadata?
    pub runtime: Runtime,
}

impl AstMetadata {
    pub fn new(runtime: Runtime) -> Self {
        AstMetadata {
            variables: Vec::new(),
            runtime,
        }
    }
    pub fn variable_metadata(&self, id: usize) -> Option<&VariableMetadata> {
        self.variables.get(id)
    }

    pub fn variable_metadata_mut(
        &mut self,
        id: usize,
    ) -> Option<&mut VariableMetadata> {
        self.variables.get_mut(id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RichAst {
    pub ast: Option<DatexExpression>,
    pub metadata: Rc<RefCell<AstMetadata>>,
}

#[derive(Default, Debug, Clone)]
pub struct PrecompilerScope {
    pub realm_index: usize,
    pub variable_ids_by_name: HashMap<String, usize>,
}

impl PrecompilerScope {
    pub fn new_with_realm_index(realm_index: usize) -> Self {
        PrecompilerScope {
            realm_index,
            variable_ids_by_name: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableShape {
    Type,
    Value(VariableKind),
}

impl From<VariableKind> for VariableShape {
    fn from(value: VariableKind) -> Self {
        VariableShape::Value(value)
    }
}

impl Display for VariableShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableShape::Type => write!(f, "type"),
            VariableShape::Value(kind) => write!(f, "{kind}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrecompilerScopeStack {
    pub scopes: Vec<PrecompilerScope>,
}

impl Default for PrecompilerScopeStack {
    fn default() -> Self {
        PrecompilerScopeStack {
            scopes: vec![PrecompilerScope::default()],
        }
    }
}

impl PrecompilerScopeStack {
    pub fn push_scope(&mut self) {
        self.scopes.push(PrecompilerScope::new_with_realm_index(
            self.scopes.last().map_or(0, |s| s.realm_index),
        ));
    }

    pub fn pop_scope(&mut self) {
        if !self.scopes.is_empty() {
            self.scopes.pop();
        } else {
            unreachable!("Cannot pop scope from an empty scope stack");
        }
    }

    /// increment the current scope's realm index (e.g. inside a remote execution call or function body)
    pub fn increment_realm_index(&mut self) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.realm_index += 1;
        } else {
            unreachable!("Scope stack must always have at least one scope");
        }
    }

    pub fn current_realm_index(&self) -> usize {
        self.scopes.last().map_or(0, |s| s.realm_index)
    }

    pub fn add_new_variable(
        &mut self,
        name: String,
        id: usize,
        kind: VariableShape,
    ) -> VariableMetadata {
        let current_realm_index =
            self.scopes.last().map_or(0, |s| s.realm_index);
        let var_metadata = VariableMetadata {
            is_cross_realm: false,
            original_realm_index: current_realm_index,
            shape: kind,
            var_type: None,
            name: name.clone(),
        };
        self.set_variable(name, id);
        var_metadata
    }

    pub fn get_variable_and_update_metadata(
        &self,
        name: &str,
        metadata: &mut AstMetadata,
    ) -> Result<usize, CompilerError> {
        // try to resolve local variable
        if let Some(var_id) = self.get_variable(name) {
            let var_metadata = metadata.variable_metadata_mut(var_id).unwrap();
            // if the original realm index is not the current realm index, mark it as cross-realm
            info!(
                "Get variable {name} with realm index: {}, current realm index: {}",
                var_metadata.original_realm_index,
                self.current_realm_index()
            );
            if var_metadata.original_realm_index != self.current_realm_index() {
                var_metadata.is_cross_realm = true;
            }
            Ok(var_id)
        } else {
            Err(CompilerError::UndeclaredVariable(name.to_string()))
        }
    }

    pub fn set_variable(&mut self, name: String, id: usize) {
        // get the second last scope or the last one if there is only one scope
        let index = if self.scopes.len() > 1 {
            self.scopes.len() - 2
        } else {
            self.scopes.len() - 1
        };
        if let Some(scope) = self.scopes.get_mut(index) {
            scope.variable_ids_by_name.insert(name, id);
        } else {
            unreachable!("Scope stack must always have at least one scope");
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.variable_ids_by_name.get(name) {
                return Some(*id);
            }
        }
        None
    }
    pub fn has_variable(&self, name: &str) -> bool {
        self.get_variable(name).is_some()
    }

    pub fn metadata<'a>(
        &self,
        name: &str,
        metadata: &'a AstMetadata,
    ) -> Option<&'a VariableMetadata> {
        if let Some(var_id) = self.get_variable(name) {
            metadata.variable_metadata(var_id)
        } else {
            None
        }
    }
    pub fn variable_kind(
        &self,
        name: &str,
        metadata: &AstMetadata,
    ) -> Option<VariableShape> {
        if let Some(var_id) = self.get_variable(name) {
            metadata.variable_metadata(var_id).map(|v| v.shape)
        } else {
            None
        }
    }
}

impl RichAst {
    pub fn new(
        ast: DatexExpression,
        metadata: &Rc<RefCell<AstMetadata>>,
    ) -> Self {
        RichAst {
            ast: Some(ast),
            metadata: metadata.clone(),
        }
    }

    pub fn new_without_metadata(ast: DatexExpression) -> Self {
        RichAst {
            ast: Some(ast),
            metadata: Rc::new(RefCell::new(AstMetadata::default())),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PrecompilerOptions {
    /// If enabled, all collected errors as well as the RichAst
    /// are returned if one or multiple errors occurred.
    /// Otherwise, only the first error is returned (fast failing)
    pub detailed_errors: bool,
}

pub fn precompile_ast_simple_error(
    parse_result: ValidDatexParseResult,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope_stack: &mut PrecompilerScopeStack,
) -> Result<RichAst, SpannedCompilerError> {
    precompile_ast(
        parse_result,
        ast_metadata,
        scope_stack,
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

pub fn precompile_ast_detailed_error(
    parse_result: ValidDatexParseResult,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope_stack: &mut PrecompilerScopeStack,
) -> Result<RichAst, DetailedCompilerErrorsWithRichAst> {
    precompile_ast(
        parse_result,
        ast_metadata,
        scope_stack,
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

pub(crate) fn precompile_ast(
    mut parse_result: ValidDatexParseResult,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope_stack: &mut PrecompilerScopeStack,
    options: PrecompilerOptions,
) -> Result<RichAst, SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst> {
    // visit all expressions recursively to collect metadata
    let collected_errors = if options.detailed_errors {
        &mut Some(DetailedCompilerErrors::default())
    } else {
        &mut None
    };
    visit_expression(
        &mut parse_result.ast,
        &mut ast_metadata.borrow_mut(),
        scope_stack,
        NewScopeType::None,
        &parse_result.spans,
        collected_errors,
    )
    // no detailed error collection, return no RichAst 
    // TODO #486: make sure Err result is actually only returned when detailed_errors is set to false
    .map_err(SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple)?;

    let mut rich_ast = RichAst {
        metadata: ast_metadata,
        ast: Some(parse_result.ast),
    };

    // type inference - currently only if detailed errors are enabled
    // FIXME #487: always do type inference here, not only for detailed errors
    if options.detailed_errors {
        let type_res = infer_expression_type_detailed_errors(
            rich_ast.ast.as_mut().unwrap(),
            rich_ast.metadata.clone(),
        );

        // append type errors to collected_errors if any
        if let Some(collected_errors) = collected_errors
            && let Err(type_errors) = type_res
        {
            collected_errors.append(type_errors.into());
        }
    }

    // if collecting detailed errors and an error occurred, return
    if let Some(errors) = collected_errors.take()
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

enum NewScopeType {
    // no new scope, just continue in the current scope
    None,
    // create a new scope, but do not increment the realm index
    NewScope,
    // create a new scope and increment the realm index (e.g. for remote execution calls)
    NewScopeWithNewRealm,
}

/// This method must hold the contract that it always returns an Ok()
/// result if collected_errors is Some, and only returns Err() if collected_errors is None.
fn visit_expression(
    expression: &mut DatexExpression,
    metadata: &mut AstMetadata,
    scope_stack: &mut PrecompilerScopeStack,
    new_scope: NewScopeType,
    spans: &Vec<Range<usize>>,
    collected_errors: &mut Option<DetailedCompilerErrors>,
) -> Result<(), SpannedCompilerError> {
    match new_scope {
        NewScopeType::NewScopeWithNewRealm => {
            scope_stack.push_scope();
            scope_stack.increment_realm_index();
        }
        NewScopeType::NewScope => {
            scope_stack.push_scope();
        }
        _ => {}
    }

    // update span from token span -> source code span
    let span_start = expression.span.start;
    let span_end = expression.span.end;
    // skip if both zero (default span used for testing)
    // TODO #488: improve this
    if span_start != 0 || span_end != 0 {
        let start_token = spans.get(span_start).cloned().unwrap();
        let end_token = spans.get(span_end - 1).cloned().unwrap();
        let full_span = start_token.start..end_token.end;
        expression.span = SimpleSpan::from(full_span);
    }

    // Important: always make sure all expressions are visited recursively
    match &mut expression.data {
        // DatexExpression::GenericAssessor(left, right) => {
        //     visit_expression(
        //         left,
        //         metadata,
        //         scope_stack,
        //         NewScopeType::NewScope,
        //     )?;
        //     visit_expression(
        //         right,
        //         metadata,
        //         scope_stack,
        //         NewScopeType::NewScope,
        //     )?;
        // }
        DatexExpressionData::TypeExpression(type_expr) => {
            visit_type_expression(
                type_expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
            )?;
        }
        DatexExpressionData::Conditional(Conditional {
            condition,
            then_branch,
            else_branch,
        }) => {
            visit_expression(
                condition,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            visit_expression(
                then_branch,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            if let Some(else_branch) = else_branch {
                visit_expression(
                    else_branch,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                    collected_errors,
                )?;
            }
        }
        DatexExpressionData::TypeDeclaration(TypeDeclaration {
            id,
            // generic: generic_parameters,
            name,
            value,
            hoisted,
        }) => {
            visit_type_expression(
                value,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
            )?;
            // already declared if hoisted
            if *hoisted {
                *id = Some(
                    scope_stack
                        .get_variable_and_update_metadata(name, metadata)?,
                );
            } else {
                *id = Some(add_new_variable(
                    name.clone(),
                    VariableShape::Type,
                    metadata,
                    scope_stack,
                ));
            }
        }
        DatexExpressionData::VariableDeclaration(VariableDeclaration {
            id,
            kind,
            name,
            init_expression: value,
            type_annotation,
        }) => {
            visit_expression(
                value,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            if let Some(type_annotation) = type_annotation {
                visit_type_expression(
                    type_annotation,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                )?;
            }
            *id = Some(add_new_variable(
                name.clone(),
                VariableShape::Value(*kind),
                metadata,
                scope_stack,
            ));
        }
        DatexExpressionData::Identifier(name) => {
            let action = collect_or_pass_error(
                collected_errors,
                resolve_variable(name, metadata, scope_stack).map_err(
                    |error| {
                        SpannedCompilerError::new_with_simple_span(
                            error,
                            expression.span,
                        )
                    },
                ),
            )?;
            if let MaybeAction::Do(resolved_variable) = action {
                *expression = match resolved_variable {
                    ResolvedVariable::VariableId(id) => {
                        DatexExpressionData::VariableAccess(VariableAccess {
                            id,
                            name: name.clone(),
                        })
                        .with_span(expression.span)
                    }
                    ResolvedVariable::PointerAddress(pointer_address) => {
                        DatexExpressionData::GetReference(pointer_address)
                            .with_span(expression.span)
                    }
                };
            }
        }
        DatexExpressionData::VariableAssignment(VariableAssignment {
            id,
            name,
            expression: inner_expression,
            ..
        }) => {
            visit_expression(
                inner_expression,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            let new_id =
                scope_stack.get_variable_and_update_metadata(name, metadata)?;
            // check if variable is const
            let var_metadata = metadata
                .variable_metadata(new_id)
                .expect("Variable must have metadata");
            if let VariableShape::Value(VariableKind::Const) =
                var_metadata.shape
            {
                let error = SpannedCompilerError::new_with_simple_span(
                    CompilerError::AssignmentToConst(name.clone()),
                    expression.span,
                );
                match collected_errors {
                    Some(collected_errors) => {
                        collected_errors.record_error(error);
                    }
                    None => return Err(error),
                }
            }
            *id = Some(new_id);
        }
        DatexExpressionData::DerefAssignment(DerefAssignment {
            deref_expression,
            assigned_expression,
            ..
        }) => {
            visit_expression(
                deref_expression,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            visit_expression(
                assigned_expression,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::Deref(expr) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::ApplyChain(ApplyChain {
            base: expr,
            operations: applies,
        }) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            for apply in applies {
                match apply {
                    ApplyOperation::FunctionCall(expr)
                    | ApplyOperation::GenericAccess(expr)
                    | ApplyOperation::PropertyAccess(expr) => {
                        visit_expression(
                            expr,
                            metadata,
                            scope_stack,
                            NewScopeType::NewScope,
                            spans,
                            collected_errors,
                        )?;
                    }
                }
            }
        }
        DatexExpressionData::List(exprs) => {
            for expr in &mut exprs.items {
                visit_expression(
                    expr,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                    collected_errors,
                )?;
            }
        }
        DatexExpressionData::Map(properties) => {
            for (key, val) in &mut properties.entries {
                visit_expression(
                    key,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                    collected_errors,
                )?;
                visit_expression(
                    val,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                    collected_errors,
                )?;
            }
        }
        DatexExpressionData::RemoteExecution(RemoteExecution {
            left: callee,
            right: expr,
        }) => {
            visit_expression(
                callee,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScopeWithNewRealm,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::BinaryOperation(BinaryOperation {
            operator,
            left,
            right,
            ..
        }) => {
            if matches!(operator, BinaryOperator::VariantAccess) {
                let lit_left =
                    if let DatexExpressionData::Identifier(name) = &left.data {
                        name.clone()
                    } else {
                        unreachable!(
                            "Left side of variant access must be a literal"
                        );
                    };

                let lit_right = if let DatexExpressionData::Identifier(name) =
                    &right.data
                {
                    name.clone()
                } else {
                    unreachable!(
                        "Right side of variant access must be a literal"
                    );
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
                if scope_stack.has_variable(lit_left.as_str()) {
                    match scope_stack
                        .variable_kind(lit_left.as_str(), metadata)
                        .unwrap()
                    {
                        VariableShape::Type => {
                            // user defined type, continue to variant access
                            let resolved_variable = resolve_variable(
                                &full_name,
                                metadata,
                                scope_stack,
                            )
                            .map_err(|_| {
                                CompilerError::SubvariantNotFound(
                                    lit_left.to_string(),
                                    lit_right.to_string(),
                                )
                            })?;
                            *expression = match resolved_variable {
                                ResolvedVariable::VariableId(id) => {
                                    DatexExpressionData::VariableAccess(
                                        VariableAccess {
                                            id,
                                            name: full_name.to_string(),
                                        },
                                    )
                                    .with_span(expression.span)
                                }
                                _ => unreachable!(
                                    "Variant access must resolve to a core library type"
                                ),
                            };
                        }
                        VariableShape::Value(_) => {
                            // user defined value, this is a division
                            visit_expression(
                                left,
                                metadata,
                                scope_stack,
                                NewScopeType::NewScope,
                                spans,
                                collected_errors,
                            )?;
                            visit_expression(
                                right,
                                metadata,
                                scope_stack,
                                NewScopeType::NewScope,
                                spans,
                                collected_errors,
                            )?;

                            *expression = DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::Arithmetic(
                                        ArithmeticOperator::Divide,
                                    ),
                                    left: left.to_owned(),
                                    right: right.to_owned(),
                                    r#type: None,
                                },
                            )
                            .with_span(expression.span);
                        }
                    }
                    return Ok(());
                }
                // can be either a core type or a undeclared variable

                // check if left part is a core value / type
                // otherwise throw the error
                resolve_variable(lit_left.as_str(), metadata, scope_stack)?;

                let resolved_variable = resolve_variable(
                    format!("{lit_left}/{lit_right}").as_str(),
                    metadata,
                    scope_stack,
                )
                .map_err(|error| {
                    SpannedCompilerError::new_with_simple_span(
                        CompilerError::SubvariantNotFound(lit_left, lit_right),
                        expression.span,
                    )
                });
                let action =
                    collect_or_pass_error(collected_errors, resolved_variable)?;
                if let MaybeAction::Do(resolved_variable) = action {
                    *expression = match resolved_variable {
                        ResolvedVariable::PointerAddress(pointer_address) => {
                            DatexExpressionData::GetReference(pointer_address)
                                .with_span(expression.span)
                        }
                        // FIXME #442 is variable User/whatever allowed here, or
                        // will this always be a reference to the type?
                        _ => unreachable!(
                            "Variant access must resolve to a core library type"
                        ),
                    };
                    return Ok(());
                }
            }

            visit_expression(
                left,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            visit_expression(
                right,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::UnaryOperation(UnaryOperation {
            operator: _,
            expression,
        }) => {
            visit_expression(
                expression,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::SlotAssignment(SlotAssignment {
            expression,
            ..
        }) => {
            visit_expression(
                expression,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::GetReference(_pointer_id) => {
            // nothing to do
        }
        DatexExpressionData::Statements(stmts) => {
            // hoist type declarations first
            let mut registered_names = HashSet::new();
            for stmt in stmts.statements.iter_mut() {
                if let DatexExpressionData::TypeDeclaration(TypeDeclaration {
                    name,
                    hoisted,
                    ..
                }) = &mut stmt.data
                {
                    // set hoisted to true
                    *hoisted = true;
                    if registered_names.contains(name) {
                        let error = SpannedCompilerError::new_with_simple_span(
                            CompilerError::InvalidRedeclaration(name.clone()),
                            stmt.span,
                        );
                        match collected_errors {
                            Some(collected_errors) => {
                                collected_errors.record_error(error);
                            }
                            None => return Err(error),
                        }
                    }
                    registered_names.insert(name.clone());

                    // register variable
                    let type_id = add_new_variable(
                        name.clone(),
                        VariableShape::Type,
                        metadata,
                        scope_stack,
                    );

                    // register placeholder ref in metadata
                    let reference =
                        Rc::new(RefCell::new(TypeReference::nominal(
                            Type::UNIT,
                            NominalTypeDeclaration::from(name.to_string()),
                            None,
                        )));
                    let type_def =
                        TypeContainer::TypeReference(reference.clone());
                    {
                        metadata
                            .variable_metadata_mut(type_id)
                            .expect(
                                "TypeDeclaration should have variable metadata",
                            )
                            .var_type = Some(type_def.clone());
                    }
                }
            }
            for stmt in &mut stmts.statements {
                visit_expression(
                    stmt,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                    collected_errors,
                )?
            }
        }
        DatexExpressionData::ComparisonOperation(ComparisonOperation {
            left,
            right,
            ..
        }) => {
            visit_expression(
                left,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
            visit_expression(
                right,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::CreateRefMut(expr)
        | DatexExpressionData::CreateRefFinal(expr)
        | DatexExpressionData::CreateRef(expr) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
                collected_errors,
            )?;
        }
        DatexExpressionData::Recover => {
            unreachable!("Expression should have been caught during parsing")
        }
        DatexExpressionData::VariableAccess(_) => unreachable!(
            "Variable expressions should have been replaced with their IDs during precompilation"
        ),
        DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
            name,
            parameters,
            return_type,
            body,
        }) => todo!("#443 Undescribed by author."),

        DatexExpressionData::Integer(_)
        | DatexExpressionData::Text(_)
        | DatexExpressionData::Boolean(_)
        | DatexExpressionData::Null
        | DatexExpressionData::Decimal(_)
        | DatexExpressionData::Endpoint(_)
        | DatexExpressionData::Placeholder
        | DatexExpressionData::TypedDecimal(_)
        | DatexExpressionData::TypedInteger(_)
        | DatexExpressionData::Type(_)
        | DatexExpressionData::Slot(_)
        | DatexExpressionData::PointerAddress(_) => {
            // ignored
        }
    }

    match new_scope {
        NewScopeType::NewScope | NewScopeType::NewScopeWithNewRealm => {
            scope_stack.pop_scope();
        }
        _ => {}
    }

    Ok(())
}

fn add_new_variable(
    name: String,
    kind: VariableShape,
    metadata: &mut AstMetadata,
    scope_stack: &mut PrecompilerScopeStack,
) -> usize {
    let new_id = metadata.variables.len();
    let var_metadata = scope_stack.add_new_variable(name.clone(), new_id, kind);
    metadata.variables.push(var_metadata);
    new_id
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ResolvedVariable {
    VariableId(usize),
    PointerAddress(PointerAddress),
}

/// Resolves a variable name to either a local variable ID if it was already declared (or hoisted),
/// or to a core library pointer ID if it is a core variable.
/// If the variable cannot be resolved, a CompilerError is returned.
fn resolve_variable(
    name: &str,
    metadata: &mut AstMetadata,
    scope_stack: &mut PrecompilerScopeStack,
) -> Result<ResolvedVariable, CompilerError> {
    // If variable exist
    if let Ok(id) = scope_stack.get_variable_and_update_metadata(name, metadata)
    {
        info!("Visiting variable: {name}, scope stack: {scope_stack:?}");
        Ok(ResolvedVariable::VariableId(id))
    }
    // try to resolve core variable
    else if let Some(core) = metadata
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

// FIXME #489: use tree visitor once fully implemented instead of custom visit function
fn visit_type_expression(
    type_expr: &mut TypeExpressionData,
    metadata: &mut AstMetadata,
    scope_stack: &mut PrecompilerScopeStack,
    new_scope: NewScopeType,
    spans: &Vec<Range<usize>>,
) -> Result<(), CompilerError> {
    match type_expr {
        TypeExpressionData::Literal(name) => {
            let resolved_variable =
                resolve_variable(name, metadata, scope_stack)?;
            *type_expr = match resolved_variable {
                ResolvedVariable::VariableId(id) => {
                    TypeExpressionData::Variable(id, name.clone())
                }
                ResolvedVariable::PointerAddress(pointer_address) => {
                    TypeExpressionData::GetReference(pointer_address)
                }
            };
            Ok(())
        }
        TypeExpressionData::Integer(_)
        | TypeExpressionData::Text(_)
        | TypeExpressionData::Boolean(_)
        | TypeExpressionData::Null
        | TypeExpressionData::Decimal(_)
        | TypeExpressionData::Endpoint(_)
        | TypeExpressionData::TypedDecimal(_)
        | TypeExpressionData::TypedInteger(_)
        | TypeExpressionData::GetReference(_) => Ok(()),
        TypeExpressionData::StructuralList(inner_type) => {
            for ty in inner_type {
                visit_type_expression(
                    ty,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                )?;
            }
            Ok(())
        }
        TypeExpressionData::StructuralMap(properties) => {
            for (_, ty) in properties {
                visit_type_expression(
                    ty,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                )?;
            }
            Ok(())
        }
        TypeExpressionData::Union(types) => {
            for ty in types {
                visit_type_expression(
                    ty,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                )?;
            }
            Ok(())
        }
        TypeExpressionData::Intersection(types) => {
            for ty in types {
                visit_type_expression(
                    ty,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                    spans,
                )?;
            }
            Ok(())
        }
        TypeExpressionData::RefMut(inner) | TypeExpressionData::Ref(inner) => {
            visit_type_expression(
                inner,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
                spans,
            )?;
            Ok(())
        }
        e => todo!(
            "{}",
            format!(
                "#445 Handle other type expressions in precompiler: {:?}",
                e
            )
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parse_result::{DatexParseResult, InvalidDatexParseResult};
    use crate::ast::tree::Statements;
    use crate::ast::{error::src::SrcId, parse};
    use crate::runtime::RuntimeConfig;
    use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
    use datex_core::values::core_values::integer::Integer;
    use std::assert_matches::assert_matches;
    use std::io;

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
            panic!("Parsing errors found");
        }
        res.unwrap().ast
    }

    fn parse_and_precompile_spanned_result(
        src: &str,
    ) -> Result<RichAst, SpannedCompilerError> {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::new(runtime)));
        let expr = parse(src)
            .to_result()
            .map_err(|mut e| SpannedCompilerError::from(e.remove(0)))?;
        precompile_ast(
            expr,
            ast_metadata.clone(),
            &mut scope_stack,
            PrecompilerOptions {
                detailed_errors: false,
            },
        )
        .map_err(|e| match e {
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(
                error,
            ) => error,
            _ => unreachable!(), // because detailed_errors: false
        })
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
        assert_matches!(
            result,
            Ok(
                RichAst {
                    ast: Some(DatexExpression { data: DatexExpressionData::GetReference(pointer_id), ..}),
                    ..
                }
            ) if pointer_id == CoreLibPointerId::Integer(Some(IntegerTypeVariant::U8)).into()
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
                DatexExpressionData::GetReference(
                    CoreLibPointerId::Integer(Some(IntegerTypeVariant::U8))
                        .into()
                )
                .with_default_span()
            )
        );

        // core type with bad variant should error
        let result = parse_and_precompile("integer/invalid");
        assert_matches!(result, Err(CompilerError::SubvariantNotFound(name, variant)) if name == "integer" && variant == "invalid");

        // unknown type should error
        let result = parse_and_precompile("invalid/u8");
        assert_matches!(result, Err(CompilerError::UndeclaredVariable(var_name)) if var_name == "invalid");

        // declared type with invalid subvariant shall throw
        let result = parse_and_precompile("type User = {}; User/u8");
        assert!(result.is_err());
        assert_matches!(result, Err(CompilerError::SubvariantNotFound(name, variant)) if name == "User" && variant == "u8");

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
                            value: TypeExpressionData::StructuralMap(vec![]),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(1),
                            name: "User/admin".to_string(),
                            value: TypeExpressionData::StructuralMap(vec![]),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::VariableAccess(VariableAccess {
                            id: 1,
                            name: "User/admin".to_string()
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
            panic!("Expected statements");
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
            panic!("Expected statements");
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
                            )),
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
                            )),
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
                            value: TypeExpressionData::Variable(
                                1,
                                "MyInt".to_string()
                            ),
                            hoisted: true,
                        })
                        .with_default_span(),
                        DatexExpressionData::TypeDeclaration(TypeDeclaration {
                            id: Some(1),
                            name: "MyInt".to_string(),
                            value: TypeExpressionData::Variable(
                                0,
                                "x".to_string()
                            ),
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
                            ),
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
                                        value: TypeExpressionData::Variable(
                                            0,
                                            "x".to_string()
                                        ),
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
                    ),
                    hoisted: false,
                })
                .with_default_span()
            )
        );
    }
}
