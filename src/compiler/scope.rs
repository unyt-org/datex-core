use std::cell::RefCell;
use crate::compiler::{ast_parser::VariableType, context::VirtualSlot};
use std::collections::HashMap;
use std::rc::Rc;
use crate::compiler::ast_parser::VariableMutType;
use crate::compiler::precompiler::{AstMetadata, PrecompilerScopeStack};


#[derive(Debug, Clone, Default)]
pub struct PrecompilerData {

    // precompiler ast metadata
    pub ast_metadata: Rc<RefCell<AstMetadata>>,
    // precompiler scope stack
    pub precompiler_scope_stack: RefCell<PrecompilerScopeStack>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    /// List of variables, mapped by name to their slot address and type.
    variables: HashMap<String, (u32, VariableType, VariableMutType)>,
    /// parent scope, accessible from a child scope
    parent_scope: Option<Box<Scope>>,
    /// scope of a parent context, e.g. when inside a block scope for remote execution calls or function bodies
    external_parent_scope: Option<Box<Scope>>,
    next_slot_address: u32,

    /// optional precompiler data, only on the root scope
    pub precompiler_data: Option<PrecompilerData>,
}

impl Default for Scope {
    fn default() -> Self {
        Scope {
            variables: HashMap::new(),
            parent_scope: None,
            external_parent_scope: None,
            next_slot_address: 0,
            precompiler_data: Some(PrecompilerData::default()),
        }
    }
}


impl Scope {
    
    pub fn new_with_external_parent_scope(parent_context: Scope) -> Scope {
        Scope {
            external_parent_scope: Some(Box::new(parent_context)),
            ..Scope::default()
        }
    }

    pub fn has_external_parent_scope(&self) -> bool {
        self.external_parent_scope.is_some()
    }

    pub fn register_variable_slot(
        &mut self,
        slot_address: u32,
        variable_type: VariableType,
        mut_type: VariableMutType,
        name: String,
    ) {
        self.variables
            .insert(name.clone(), (slot_address, variable_type, mut_type));
    }

    pub fn get_next_virtual_slot(&mut self) -> u32 {
        let slot_address = self.next_slot_address;
        self.next_slot_address += 1;
        slot_address
    }

    // Returns the virtual slot address for a variable in this scope or potentially in the parent scope.
    // The returned tuple contains the slot address, variable type, and a boolean indicating if it
    // is a local variable (false) or from a parent scope (true).
    pub fn resolve_variable_name_to_virtual_slot(
        &self,
        name: &str,
    ) -> Option<(VirtualSlot, VariableType, VariableMutType)> {
        if let Some(slot) = self.variables.get(name) {
            Some((VirtualSlot::local(slot.0), slot.1, slot.2))
        } else if let Some(external_parent) = &self.external_parent_scope {
            external_parent
                .resolve_variable_name_to_virtual_slot(name)
                .map(|(virt_slot, var_type, mut_type)| (virt_slot.downgrade(), var_type, mut_type))
        } else if let Some(parent) = &self.parent_scope {
            parent
                .resolve_variable_name_to_virtual_slot(name)
        } else {
            None
        }
    }

    /// Creates a new `CompileScope` that is a child of the current scope.
    pub fn push(self) -> Scope {
        Scope {
            next_slot_address: self.next_slot_address,
            parent_scope: Some(Box::new(self)),
            external_parent_scope: None,
            variables: HashMap::new(),
            precompiler_data: None,
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
