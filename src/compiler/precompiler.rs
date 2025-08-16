use crate::compiler::ast_parser::{Apply, DatexExpression, TupleEntry};
use crate::compiler::error::CompilerError;
use log::info;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct VariableMetadata {
    original_realm_index: usize,
    pub is_cross_realm: bool,
    // TODO #239: store type information etc.
}

#[derive(Default, Debug)]
pub struct AstMetadata {
    pub variables: Vec<VariableMetadata>,
}

impl AstMetadata {
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
    ) -> VariableMetadata {
        let current_realm_index =
            self.scopes.last().map_or(0, |s| s.realm_index);
        let var_metadata = VariableMetadata {
            is_cross_realm: false,
            original_realm_index: current_realm_index,
        };
        self.set_variable(name, id);
        var_metadata
    }

    pub fn get_variable_id(
        &self,
        name: &str,
        metadata: &mut AstMetadata,
    ) -> Result<usize, CompilerError> {
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
        DatexExpression::VariableDeclaration(
            id,
            var_type,
            binding_mut,
            ref_mut,
            name,
            expr,
        ) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            let new_id = metadata.variables.len();
            *id = Some(new_id);
            let var_metadata =
                scope_stack.add_new_variable(name.clone(), new_id);
            metadata.variables.push(var_metadata);
        }

        DatexExpression::Variable(id, name) => {
            info!("Visiting variable: {name}, scope stack: {scope_stack:?}");
            *id = Some(scope_stack.get_variable_id(name, metadata)?);
        }

        DatexExpression::VariableAssignment(id, name, expr) => {
            visit_expression(
                expr,
                metadata,
                scope_stack,
                NewScopeType::NewScope,
            )?;
            *id = Some(scope_stack.get_variable_id(name, metadata)?);
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
                    Apply::FunctionCall(expr) => {
                        visit_expression(
                            expr,
                            metadata,
                            scope_stack,
                            NewScopeType::NewScope,
                        )?;
                    }
                    Apply::PropertyAccess(expr) => {
                        visit_expression(
                            expr,
                            metadata,
                            scope_stack,
                            NewScopeType::NewScope,
                        )?;
                    }
                }
            }
        }

        DatexExpression::Array(exprs) => {
            for expr in exprs {
                visit_expression(
                    expr,
                    metadata,
                    scope_stack,
                    NewScopeType::NewScope,
                )?;
            }
        }

        DatexExpression::Object(properties) => {
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

        DatexExpression::Tuple(entries) => {
            for entry in entries {
                match entry {
                    TupleEntry::Value(expr) => {
                        visit_expression(
                            expr,
                            metadata,
                            scope_stack,
                            NewScopeType::NewScope,
                        )?;
                    }
                    TupleEntry::KeyValue(key, value) => {
                        visit_expression(
                            key,
                            metadata,
                            scope_stack,
                            NewScopeType::NewScope,
                        )?;
                        visit_expression(
                            value,
                            metadata,
                            scope_stack,
                            NewScopeType::NewScope,
                        )?;
                    }
                }
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

        DatexExpression::BinaryOperation(_operator, left, right) => {
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

        DatexExpression::Statements(stmts) => {
            for stmt in stmts {
                visit_expression(
                    &mut stmt.expression,
                    metadata,
                    scope_stack,
                    NewScopeType::None,
                )?;
            }
        }

        _ => {}
    }

    match new_scope {
        NewScopeType::NewScope | NewScopeType::NewScopeWithNewRealm => {
            scope_stack.pop_scope();
        }
        _ => {}
    }

    Ok(())
}
