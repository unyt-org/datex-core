use crate::compiler::ast_parser::VariableType;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// List of variables, mapped by name to their slot address and type.
    variables: HashMap<String, (u32, VariableType)>,
    parent_scope: Option<Box<Scope>>,
    next_slot_address: u32,
}

impl Scope {
    pub fn register_variable_slot(
        &mut self,
        slot_address: u32,
        variable_type: VariableType,
        name: String,
    ) {
        self.variables
            .insert(name.clone(), (slot_address, variable_type));
    }

    pub fn get_next_variable_slot(&mut self) -> u32 {
        let slot_address = self.next_slot_address;
        self.next_slot_address += 1;
        slot_address
    }

    pub fn resolve_variable_slot(
        &self,
        name: &str,
    ) -> Option<(u32, VariableType)> {
        let mut variables = &self.variables;
        loop {
            if let Some(slot) = variables.get(name) {
                return Some(slot.clone());
            }
            if let Some(parent) = &self.parent_scope {
                variables = &parent.variables;
            } else {
                return None; // variable not found in this scope or any parent scope
            }
        }
    }

    /// Creates a new `CompileScope` that is a child of the current scope.
    pub fn push(self) -> Scope {
        Scope {
            next_slot_address: self.next_slot_address,
            parent_scope: Some(Box::new(self)),
            variables: HashMap::new(),
        }
    }

    /// Drops the current scope and returns to the parent scope and a list
    /// of all slot addresses that should be dropped.
    pub fn pop(self) -> Option<(Scope, Vec<u32>)> {
        if let Some(mut parent) = self.parent_scope {
            // update next_slot_address for parent scope
            parent.next_slot_address = self.next_slot_address;
            Some((
                *parent,
                self.variables.keys().map(|k| self.variables[k].0).collect(),
            ))
        } else {
            None
        }
    }
}
