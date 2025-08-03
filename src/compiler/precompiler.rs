use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use log::info;
use crate::compiler::ast_parser::{Apply, DatexExpression, TupleEntry};
use crate::compiler::error::CompilerError;

#[derive(Clone, Debug, Default)]
pub struct VariableMetadata {
    is_cross_realm: bool,
    // TODO: store type information etc.
}

#[derive(Default, Debug)]
pub struct AstMetadata {
    pub variables: Vec<VariableMetadata>,
}

pub struct AstWithMetadata {
    pub ast: DatexExpression,
    pub metadata: Rc<RefCell<AstMetadata>>,
}

#[derive(Default, Debug, Clone)]
pub struct PrecompilerScope {
    pub variable_ids_by_name: HashMap<String, usize>,
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
        self.scopes.push(PrecompilerScope::default());
    }

    pub fn pop_scope(&mut self) {
        if !self.scopes.is_empty() {
            self.scopes.pop();
        } else {
            unreachable!("Cannot pop scope from an empty scope stack");
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
        }
        else {
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
    pub fn new(ast: DatexExpression, metadata: &Rc<RefCell<AstMetadata>>) -> Self {
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

pub fn precompile_ast(mut ast: DatexExpression, ast_metadata: Rc<RefCell<AstMetadata>>, scope_stack: &mut PrecompilerScopeStack) -> Result<AstWithMetadata, CompilerError> {

    // visit all expressions recursively to collect metadata
    visit_expression(&mut ast, &mut ast_metadata.borrow_mut(), scope_stack, false)?;

    Ok(AstWithMetadata {
        metadata: ast_metadata,
        ast,
    })
}

fn visit_expression(expression: &mut DatexExpression, metadata: &mut AstMetadata, scope_stack: &mut PrecompilerScopeStack, new_scope: bool) -> Result<(), CompilerError> {
    if new_scope {
        scope_stack.push_scope();
    }
    // Important: always make sure all expressions are visited recursively
    match expression {
        DatexExpression::VariableDeclaration(id, var_type, mut_type, name, expr) => {
            visit_expression(expr, metadata, scope_stack, true)?;

            let var_metadata = VariableMetadata {
                is_cross_realm: false,
            };
            let new_id = metadata.variables.len();
            *id = Some(new_id);
            metadata.variables.push(var_metadata);
            scope_stack.set_variable(name.clone(), new_id);
        },

        DatexExpression::Variable(id, name) => {
            info!("Visiting variable: {}, scope stack: {:?}", name, scope_stack);
            if let Some(scope) = scope_stack.get_variable(name) {
                *id = Some(scope);
            } else {
                return Err(CompilerError::UndeclaredVariable(name.clone()))
            }
        }

        DatexExpression::VariableAssignment(id, name, expr) => {
            visit_expression(expr, metadata, scope_stack, true)?;

            if let Some(scope) = scope_stack.get_variable(name) {
                *id = Some(scope);
            } else {
                return Err(CompilerError::UndeclaredVariable(name.clone()))
            }
            visit_expression(expr, metadata, scope_stack, true)?;
        }

        DatexExpression::ApplyChain(expr, applies) => {
            visit_expression(expr, metadata, scope_stack, true)?;
            for apply in applies {
                match apply {
                    Apply::FunctionCall(expr) => {
                        visit_expression(expr, metadata, scope_stack, true)?;
                    },
                    Apply::PropertyAccess(expr) => {
                        visit_expression(expr, metadata, scope_stack, true)?;
                    }
                }
            }
        }

        DatexExpression::Array(exprs) => {
            for expr in exprs {
                visit_expression(expr, metadata, scope_stack, true)?;
            }
        }

        DatexExpression::Object(properties) => {
            for (key, val) in properties {
                visit_expression(key, metadata, scope_stack, true)?;
                visit_expression(val, metadata, scope_stack, true)?;
            }
        }

        DatexExpression::Tuple(entries) => {
            for entry in entries {
                match entry {
                    TupleEntry::Value(expr) => {
                        visit_expression(expr, metadata, scope_stack, true)?;
                    },
                    TupleEntry::KeyValue(key, value) => {
                        visit_expression(key, metadata, scope_stack, true)?;
                        visit_expression(value, metadata, scope_stack, true)?;
                    }
                }
            }
        }

        DatexExpression::RemoteExecution(callee, expr) => {
            visit_expression(callee, metadata, scope_stack, true)?;
            visit_expression(expr, metadata, scope_stack, true)?;
        }

        DatexExpression::BinaryOperation(_operator, left, right) => {
            visit_expression(left, metadata, scope_stack, true)?;
            visit_expression(right, metadata, scope_stack, true)?;
        }

        DatexExpression::UnaryOperation(_operator, expr) => {
            visit_expression(expr, metadata, scope_stack, true)?;
        }

        DatexExpression::SlotAssignment(_slot, expr) => {
            visit_expression(expr, metadata, scope_stack, true)?;
        }

        DatexExpression::Statements(stmts) => {
            for stmt in stmts {
                info!("Visiting statemtn with stack: {:?}, {:?}", scope_stack, stmt.expression);
                visit_expression(&mut stmt.expression, metadata, scope_stack, false)?;
            }
        }

        _ => {}
    }
    
    if new_scope {
        scope_stack.pop_scope();
    }

    Ok(())
}
