use log::info;

use crate::{
    compiler::error::CompilerError,
    precompiler::{
        precompiled_ast::{AstMetadata, VariableMetadata, VariableShape},
        scope::PrecompilerScope,
    },
};

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
            println!("befoere pop: {:#?}", self.scopes);
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
        println!("Setting variable {} with id {}, {:#?}", name, id, self.scopes);
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
