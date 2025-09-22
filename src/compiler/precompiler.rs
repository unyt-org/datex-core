use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::{ArithmeticOperator, BinaryOperator};
use crate::ast::chain::ApplyOperation;
use crate::ast::{DatexExpression, TypeExpression};
use crate::compiler::error::CompilerError;
use crate::libs::core::CoreLibPointerId;
use crate::runtime::Runtime;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use log::info;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct VariableMetadata {
    original_realm_index: usize,
    pub is_cross_realm: bool,
    pub kind: VariableKind,
    // TODO #239: store type information etc.
}

#[derive(Default, Debug)]
pub struct AstMetadata {
    pub variables: Vec<VariableMetadata>,
    // TODO: move runtime somewhere else, not in AstMetadata?
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

#[derive(Debug)]
pub struct AstWithMetadata {
    pub ast: DatexExpression,
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
pub enum VariableKind {
    Type,
    Value,
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
        kind: VariableKind,
    ) -> VariableMetadata {
        let current_realm_index =
            self.scopes.last().map_or(0, |s| s.realm_index);
        let var_metadata = VariableMetadata {
            is_cross_realm: false,
            original_realm_index: current_realm_index,
            kind,
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
    ) -> Option<VariableKind> {
        if let Some(var_id) = self.get_variable(name) {
            metadata.variable_metadata(var_id).map(|v| v.kind)
        } else {
            None
        }
    }
}

impl AstWithMetadata {
    pub fn new(
        ast: DatexExpression,
        metadata: &Rc<RefCell<AstMetadata>>,
    ) -> Self {
        AstWithMetadata {
            ast,
            metadata: metadata.clone(),
        }
    }

    pub fn new_without_metadata(ast: DatexExpression) -> Self {
        AstWithMetadata {
            ast,
            metadata: Rc::new(RefCell::new(AstMetadata::default())),
        }
    }
}

pub fn precompile_ast(
    mut ast: DatexExpression,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope_stack: &mut PrecompilerScopeStack,
) -> Result<AstWithMetadata, CompilerError> {
    // visit all expressions recursively to collect metadata
    visit_expression(
        &mut ast,
        &mut ast_metadata.borrow_mut(),
        scope_stack,
        NewScopeType::None,
    )?;

    Ok(AstWithMetadata {
        metadata: ast_metadata,
        ast,
    })
}

enum NewScopeType {
    // no new scope, just continue in the current scope
    None,
    // create a new scope, but do not increment the realm index
    NewScope,
    // create a new scope and increment the realm index (e.g. for remote execution calls)
    NewScopeWithNewRealm,
}

fn visit_expression(
    expression: &mut DatexExpression,
    metadata: &mut AstMetadata,
    scope_stack: &mut PrecompilerScopeStack,
    new_scope: NewScopeType,
) -> Result<(), CompilerError> {
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

    // Important: always make sure all expressions are visited recursively
    match expression {
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
        DatexExpression::TypeExpression(type_expr) => {
            visit_type_expression(
                type_expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
        }
        DatexExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_expression(
                condition,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            visit_expression(
                then_branch,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            if let Some(else_branch) = else_branch {
                visit_expression(
                    else_branch,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
            }
        }
        DatexExpression::TypeDeclaration {
            id,
            // generic: generic_parameters,
            name,
            value,
            hoisted,
        } => {
            visit_type_expression(
                value,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
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
                    VariableKind::Type,
                    metadata,
                    scope_stack,
                ));
            }
        }
        DatexExpression::VariableDeclaration {
            id,
            kind,
            binding_mutability,
            name,
            value,
            type_annotation,
        } => {
            visit_expression(
                value,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            *id = Some(add_new_variable(
                name.clone(),
                VariableKind::Value,
                metadata,
                scope_stack,
            ));
        }
        DatexExpression::Literal(name) => {
            let resolved_variable =
                resolve_variable(name, metadata, scope_stack)?;
            *expression = match resolved_variable {
                ResolvedVariable::VariableId(id) => {
                    DatexExpression::Variable(id, name.clone())
                }
                ResolvedVariable::PointerId(pointer_address) => {
                    DatexExpression::GetReference(pointer_address)
                }
            };
        }
        DatexExpression::VariableAssignment(operator, id, name, expr) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            *id = Some(
                scope_stack.get_variable_and_update_metadata(name, metadata)?,
            );
        }
        DatexExpression::ApplyChain(expr, applies) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
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
                        )?;
                    }
                    ApplyOperation::ArrayType => {
                        todo!("Handle ArrayType in precompiler")
                    }
                }
            }
        }
        DatexExpression::Array(exprs) | DatexExpression::List(exprs) => {
            for expr in exprs {
                visit_expression(
                    expr,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
            }
        }
        DatexExpression::Struct(properties) => {
            for (_, val) in properties {
                visit_expression(
                    val,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
            }
        }
        DatexExpression::Map(properties) => {
            for (key, val) in properties {
                visit_expression(
                    key,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
                visit_expression(
                    val,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
            }
        }
        DatexExpression::RemoteExecution(callee, expr) => {
            visit_expression(
                callee,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScopeWithNewRealm,
            )?;
        }
        DatexExpression::BinaryOperation(operator, left, right, _) => {
            if matches!(operator, BinaryOperator::VariantAccess) {
                let lit_left = if let DatexExpression::Literal(name) = &**left {
                    name.clone()
                } else {
                    unreachable!(
                        "Left side of variant access must be a literal"
                    );
                };

                let lit_right = if let DatexExpression::Literal(name) = &**right
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
                        VariableKind::Type => {
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
                                    DatexExpression::Variable(
                                        id,
                                        full_name.to_string(),
                                    )
                                }
                                _ => unreachable!(
                                    "Variant access must resolve to a core library type"
                                ),
                            };
                        }
                        VariableKind::Value => {
                            // user defined value, this is a division
                            visit_expression(
                                left,
                                metadata,
                                scope_stack,
                                NewScopeType::NewScope,
                            )?;
                            visit_expression(
                                right,
                                metadata,
                                scope_stack,
                                NewScopeType::NewScope,
                            )?;

                            *expression = DatexExpression::BinaryOperation(
                                BinaryOperator::Arithmetic(
                                    ArithmeticOperator::Divide,
                                ),
                                left.to_owned(),
                                right.to_owned(),
                                None,
                            );
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
                );
                if resolved_variable.is_err() {
                    return Err(CompilerError::SubvariantNotFound(
                        lit_left, lit_right,
                    ));
                }
                *expression = match resolved_variable.unwrap() {
                    ResolvedVariable::PointerId(pointer_address) => {
                        DatexExpression::GetReference(pointer_address)
                    }
                    // FIXME is variable User/whatever allowed here, or
                    // will this always be a reference to the type?
                    _ => unreachable!(
                        "Variant access must resolve to a core library type"
                    ),
                };
                return Ok(());
            }

            visit_expression(
                left,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            visit_expression(
                right,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
        }
        DatexExpression::UnaryOperation(_operator, expr) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
        }
        DatexExpression::SlotAssignment(_slot, expr) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
        }
        DatexExpression::GetReference(_pointer_id) => {
            // nothing to do
        }
        DatexExpression::Statements(stmts) => {
            // hoist type declarations first
            for stmt in stmts.iter_mut() {
                // TODO: prevent duplicate declarations
                if let DatexExpression::TypeDeclaration {
                    name, hoisted, ..
                } = &mut stmt.expression
                {
                    // set hoisted to true
                    *hoisted = true;
                    // register variable
                    add_new_variable(
                        name.clone(),
                        VariableKind::Type,
                        metadata,
                        scope_stack,
                    );
                }
            }
            for stmt in stmts {
                visit_expression(
                    &mut stmt.expression,
                    metadata,
                    scope_stack,
                    NewScopeType::None,
                )?
            }
        }
        DatexExpression::ComparisonOperation(op, left, right) => {
            visit_expression(
                left,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            visit_expression(
                right,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
        }
        DatexExpression::RefMut(expr)
        | DatexExpression::RefFinal(expr)
        | DatexExpression::Ref(expr) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
        }
        DatexExpression::Recover => {
            unreachable!("Expression should have been caught during parsing")
        }
        DatexExpression::Variable(_, _) => unreachable!(
            "Variable expressions should have been replaced with their IDs during precompilation"
        ),
        DatexExpression::FunctionDeclaration {
            name,
            parameters,
            return_type,
            body,
        } => todo!(),

        DatexExpression::Integer(_)
        | DatexExpression::Text(_)
        | DatexExpression::Boolean(_)
        | DatexExpression::Null
        | DatexExpression::Decimal(_)
        | DatexExpression::Endpoint(_)
        | DatexExpression::Placeholder
        | DatexExpression::TypedDecimal(_)
        | DatexExpression::TypedInteger(_)
        | DatexExpression::Type(_)
        | DatexExpression::Slot(_) => {
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
    kind: VariableKind,
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
    PointerId(PointerAddress),
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
        .get_reference(&CoreLibPointerId::Core.into())
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
                    Ok(ResolvedVariable::PointerId(pointer_id))
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

fn visit_type_expression(
    type_expr: &mut TypeExpression,
    metadata: &mut AstMetadata,
    scope_stack: &mut PrecompilerScopeStack,
    new_scope: NewScopeType,
) -> Result<(), CompilerError> {
    match type_expr {
        TypeExpression::Literal(name) => {
            let resolved_variable =
                resolve_variable(name, metadata, scope_stack)?;
            *type_expr = match resolved_variable {
                ResolvedVariable::VariableId(id) => {
                    TypeExpression::Variable(id, name.clone())
                }
                ResolvedVariable::PointerId(pointer_address) => {
                    TypeExpression::GetReference(pointer_address)
                }
            };
            Ok(())
        }
        TypeExpression::Integer(_)
        | TypeExpression::Text(_)
        | TypeExpression::Boolean(_)
        | TypeExpression::Null
        | TypeExpression::Decimal(_)
        | TypeExpression::Endpoint(_)
        | TypeExpression::TypedDecimal(_)
        | TypeExpression::TypedInteger(_)
        | TypeExpression::GetReference(_) => Ok(()),
        TypeExpression::Array(inner_type) => {
            for ty in inner_type {
                visit_type_expression(
                    ty,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
            }
            Ok(())
        }
        TypeExpression::Struct(properties) => {
            for (_, ty) in properties {
                visit_type_expression(
                    ty,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
            }
            Ok(())
        }
        _ => todo!("Handle other type expressions in precompiler"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Statement, error::src::SrcId, parse};
    use crate::runtime::RuntimeConfig;
    use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
    use datex_core::values::core_values::integer::integer::Integer;
    use std::assert_matches::assert_matches;
    use std::io;

    fn parse_unwrap(src: &str) -> DatexExpression {
        let src_id = SrcId::test();
        let res = parse(src);
        if let Err(errors) = res {
            errors.iter().for_each(|e| {
                let cache = ariadne::sources(vec![(src_id, src)]);
                e.clone().write(cache, io::stdout());
            });
            panic!("Parsing errors found");
        }
        res.unwrap()
    }
    fn parse_and_precompile(
        src: &str,
    ) -> Result<AstWithMetadata, CompilerError> {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::new(runtime)));
        let expr = parse_unwrap(src);
        precompile_ast(expr, ast_metadata.clone(), &mut scope_stack)
    }

    #[test]
    fn undeclared_variable() {
        let result = parse_and_precompile("x + 42");
        assert!(result.is_err());
        assert_matches!(result, Err(CompilerError::UndeclaredVariable(var_name)) if var_name == "x");
    }

    #[test]
    fn core_types() {
        let result = parse_and_precompile("boolean");
        assert_matches!(
            result,
            Ok(
                AstWithMetadata {
                    ast: DatexExpression::GetReference(pointer_id),
                    ..
                }
            ) if pointer_id == CoreLibPointerId::Boolean.into()
        );
        let result = parse_and_precompile("integer");
        assert_matches!(
            result,
            Ok(
                AstWithMetadata {
                    ast: DatexExpression::GetReference(pointer_id),
                    ..
                }
            ) if pointer_id == CoreLibPointerId::Integer(None).into()
        );

        let result = parse_and_precompile("integer/u8");
        assert_matches!(
            result,
            Ok(
                AstWithMetadata {
                    ast: DatexExpression::GetReference(pointer_id),
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
            DatexExpression::GetReference(
                CoreLibPointerId::Integer(Some(IntegerTypeVariant::U8)).into()
            )
        );

        // core type with bad variant should error
        let result = parse_and_precompile("integer/invalid");
        assert_matches!(result, Err(CompilerError::SubvariantNotFound(name, variant)) if name == "integer" && variant == "invalid");

        // unknown type should error
        let result = parse_and_precompile("unknown/u8");
        assert_matches!(result, Err(CompilerError::UndeclaredVariable(var_name)) if var_name == "unknown");

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
        let ast_with_metadata = result.unwrap();
        assert_eq!(
            ast_with_metadata.ast,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::TypeDeclaration {
                        id: Some(0),
                        name: "User".to_string(),
                        value: TypeExpression::Struct(vec![]),
                        hoisted: true,
                    },
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::TypeDeclaration {
                        id: Some(1),
                        name: "User/admin".to_string(),
                        value: TypeExpression::Struct(vec![]),
                        hoisted: true,
                    },
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Variable(
                        1,
                        "User/admin".to_string()
                    ),
                    is_terminated: false,
                }
            ])
        );

        // value shall be interpreted as division
        let result = parse_and_precompile("var a = 42; var b = 69; a/b");
        assert!(result.is_ok());
        let statements =
            if let DatexExpression::Statements(stmts) = result.unwrap().ast {
                stmts
            } else {
                panic!("Expected statements");
            };
        assert_eq!(
            statements.get(2).unwrap().expression,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::Variable(0, "a".to_string())),
                Box::new(DatexExpression::Variable(1, "b".to_string())),
                None
            )
        );

        // type with value should be interpreted as division
        let result = parse_and_precompile("var a = 10; type b = 42; a/b");
        assert!(result.is_ok());
        let statements =
            if let DatexExpression::Statements(stmts) = result.unwrap().ast {
                stmts
            } else {
                panic!("Expected statements");
            };
        assert_eq!(
            statements.get(2).unwrap().expression,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::Variable(1, "a".to_string())),
                Box::new(DatexExpression::Variable(0, "b".to_string())),
                None
            )
        );
    }

    #[test]
    fn test_type_declaration_assigment() {
        let result = parse_and_precompile("type MyInt = 1; var x = MyInt;");
        assert!(result.is_ok());
        let ast_with_metadata = result.unwrap();
        assert_eq!(
            ast_with_metadata.ast,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::TypeDeclaration {
                        id: Some(0),
                        name: "MyInt".to_string(),
                        value: TypeExpression::Integer(Integer::from(1).into()),
                        hoisted: true,
                    },
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::VariableDeclaration {
                        id: Some(1),
                        kind: crate::ast::VariableKind::Var,
                        binding_mutability:
                            crate::ast::BindingMutability::Mutable,
                        name: "x".to_string(),
                        // must refer to variable id 0
                        value: Box::new(DatexExpression::Variable(
                            0,
                            "MyInt".to_string()
                        )),
                        type_annotation: None,
                    },
                    is_terminated: true,
                },
            ])
        )
    }

    #[test]
    fn test_type_declaration_hoisted_assigment() {
        let result = parse_and_precompile("var x = MyInt; type MyInt = 1;");
        assert!(result.is_ok());
        let ast_with_metadata = result.unwrap();
        assert_eq!(
            ast_with_metadata.ast,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::VariableDeclaration {
                        id: Some(1),
                        kind: crate::ast::VariableKind::Var,
                        binding_mutability:
                            crate::ast::BindingMutability::Mutable,
                        name: "x".to_string(),
                        // must refer to variable id 0
                        value: Box::new(DatexExpression::Variable(
                            0,
                            "MyInt".to_string()
                        )),
                        type_annotation: None,
                    },
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::TypeDeclaration {
                        id: Some(0),
                        name: "MyInt".to_string(),
                        value: TypeExpression::Integer(Integer::from(1).into()),
                        hoisted: true,
                    },
                    is_terminated: true,
                },
            ])
        )
    }

    #[test]
    fn test_type_declaration_hoisted_cross_assigment() {
        let result = parse_and_precompile("type x = MyInt; type MyInt = x;");
        assert!(result.is_ok());
        let ast_with_metadata = result.unwrap();
        assert_eq!(
            ast_with_metadata.ast,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::TypeDeclaration {
                        id: Some(0),
                        name: "x".to_string(),
                        value: TypeExpression::Variable(1, "MyInt".to_string()),
                        hoisted: true,
                    },
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::TypeDeclaration {
                        id: Some(1),
                        name: "MyInt".to_string(),
                        value: TypeExpression::Variable(0, "x".to_string()),
                        hoisted: true,
                    },
                    is_terminated: true,
                },
            ])
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
        let ast_with_metadata = result.unwrap();
        assert_eq!(
            ast_with_metadata.ast,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::TypeDeclaration {
                        id: Some(0),
                        name: "x".to_string(),
                        value: TypeExpression::Integer(
                            Integer::from(10).into()
                        ),
                        hoisted: true,
                    },
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Statements(vec![
                        Statement {
                            expression: DatexExpression::Integer(
                                Integer::from(1)
                            ),
                            is_terminated: true,
                        },
                        Statement {
                            expression: DatexExpression::TypeDeclaration {
                                id: Some(1),
                                name: "NestedVar".to_string(),
                                value: TypeExpression::Variable(
                                    0,
                                    "x".to_string()
                                ),
                                hoisted: true,
                            },
                            is_terminated: true,
                        },
                    ]),
                    is_terminated: false,
                }
            ])
        )
    }

    #[test]
    fn test_core_reference_type() {
        let result = parse_and_precompile("type x = integer");
        assert!(result.is_ok());
        let ast_with_metadata = result.unwrap();
        assert_eq!(
            ast_with_metadata.ast,
            DatexExpression::TypeDeclaration {
                id: Some(0),
                name: "x".to_string(),
                value: TypeExpression::GetReference(PointerAddress::from(
                    CoreLibPointerId::Integer(None)
                )),
                hoisted: false,
            }
        );
    }
}
