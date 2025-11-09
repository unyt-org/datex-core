use crate::ast::structs::expression::VariableKind;
use crate::compiler::precompiler::precompiled_ast::RichAst;
use crate::compiler::precompiler::scope_stack::PrecompilerScopeStack;
use crate::compiler::{Variable, VariableRepresentation, context::VirtualSlot};
use crate::collections::HashMap;
use core::cell::RefCell;

#[derive(Debug, Default, Clone)]
pub struct PrecompilerData {
    // precompiler ast metadata
    pub rich_ast: RichAst,
    // precompiler scope stack
    pub precompiler_scope_stack: RefCell<PrecompilerScopeStack>,
}

#[derive(Debug, Clone)]
pub struct CompilationScope {
    /// List of variables, mapped by name to their slot address and type.
    variables: HashMap<String, Variable>,
    /// parent scope, accessible from a child scope
    parent_scope: Option<Box<CompilationScope>>,
    /// scope of a parent context, e.g. when inside a block scope for remote execution calls or function bodies
    external_parent_scope: Option<Box<CompilationScope>>,
    next_slot_address: u32,

    // ------- Data only relevant for the root scope (FIXME: refactor?) -------
    /// optional precompiler data, only on the root scope
    pub precompiler_data: Option<PrecompilerData>,
    /// If once is true, the scope can only be used for compilation once.
    /// E.g. for a REPL, this needs to be false, so that the scope can be reused
    pub once: bool,
    /// If was_used is true, the scope has been used for compilation and should not be reused if once is true.
    pub was_used: bool,
}

impl Default for CompilationScope {
    fn default() -> Self {
        CompilationScope {
            variables: HashMap::new(),
            parent_scope: None,
            external_parent_scope: None,
            next_slot_address: 0,
            precompiler_data: Some(PrecompilerData::default()),
            once: false,
            was_used: false,
        }
    }
}

impl CompilationScope {
    pub fn new(once: bool) -> CompilationScope {
        CompilationScope {
            once,
            ..CompilationScope::default()
        }
    }

    pub fn new_with_external_parent_scope(
        parent_context: CompilationScope,
    ) -> CompilationScope {
        CompilationScope {
            external_parent_scope: Some(Box::new(parent_context)),
            ..CompilationScope::default()
        }
    }

    pub fn has_external_parent_scope(&self) -> bool {
        self.external_parent_scope.is_some()
    }

    pub fn register_variable_slot(&mut self, variable: Variable) {
        self.variables.insert(variable.name.clone(), variable);
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
    ) -> Option<(VirtualSlot, VariableKind)> {
        if let Some(variable) = self.variables.get(name) {
            let slot = match variable.representation {
                VariableRepresentation::Constant(slot) => slot,
                VariableRepresentation::VariableReference {
                    container_slot,
                    ..
                } => container_slot,
                VariableRepresentation::VariableSlot(slot) => slot,
            };
            Some((slot, variable.kind))
        } else if let Some(external_parent) = &self.external_parent_scope {
            external_parent
                .resolve_variable_name_to_virtual_slot(name)
                .map(|(virt_slot, var_type)| (virt_slot.downgrade(), var_type))
        } else if let Some(parent) = &self.parent_scope {
            parent.resolve_variable_name_to_virtual_slot(name)
        } else {
            None
        }
    }

    /// Creates a new `CompileScope` that is a child of the current scope.
    pub fn push(self) -> CompilationScope {
        CompilationScope {
            next_slot_address: self.next_slot_address,
            parent_scope: Some(Box::new(self)),
            external_parent_scope: None,
            variables: HashMap::new(),
            precompiler_data: None,
            once: true,
            was_used: false,
        }
    }

    /// Drops the current scope and returns to the parent scope and a list
    /// of all slot addresses that should be dropped.
    pub fn pop(self) -> Option<(CompilationScope, Vec<VirtualSlot>)> {
        if let Some(mut parent) = self.parent_scope {
            // update next_slot_address for parent scope
            parent.next_slot_address = self.next_slot_address;
            Some((
                *parent,
                self.variables
                    .keys()
                    .flat_map(|k| self.variables[k].slots())
                    .collect::<Vec<_>>(),
            ))
        } else {
            None
        }
    }

    pub fn pop_external(self) -> Option<CompilationScope> {
        self.external_parent_scope
            .map(|external_parent| *external_parent)
    }
}
