use crate::datex_values::value_container::{ValueContainer};
use crate::global::protocol_structures::instructions::Instruction;
use crate::runtime::execution::InvalidProgramError;

// TODO: use same struct as in decompiler?
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ScopeType {
    #[default]
    Default,
    Tuple,
    Array,
    Object,
}

#[derive(Debug, Clone, Default)]
pub struct Scope {
    active_value: Option<ValueContainer>,
    scope_type: ScopeType,
}

impl Scope {
    pub fn new(scope_type: ScopeType) -> Self {
        Scope {
            scope_type,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopeStack {
    stack: Vec<Scope>,
    active_operation: Option<Instruction>
}

impl Default for ScopeStack {
    fn default() -> Self {
        ScopeStack {
            stack: vec![Scope::default()],
            active_operation: None,
        }
    }
}

impl ScopeStack {
    
    /// Returns a reference to the currently active scope.
    #[inline]
    pub fn get_current_scope(&self) -> &Scope {
        // assumes that the stack always has at least one scope
        self.stack.last().unwrap()
    }

    /// Returns a mutable reference to the currently active scope.
    #[inline]
    pub fn get_current_scope_mut(&mut self) -> &mut Scope {
        // assumes that the stack always has at least one scope
        self.stack.last_mut().unwrap()
    }
    
    /// Pops the currently active scope from the stack and return its active value
    /// If there is no active value, it returns None
    /// If there are not at least two scopes in the stack, it returns an error
    pub fn pop(&mut self) -> Result<Option<ValueContainer>, InvalidProgramError> {
        // make sure there are at least two scopes in the stack, otherwise the byte code was invalid
        if self.stack.len() < 2 {
            return Err(InvalidProgramError::InvalidScopeClose);
        }
        Ok(self.stack.pop().unwrap().active_value)
    }
    
    /// Pops the last scope from the stack and return its active value.
    /// This should only be called at the end of an execution, when extracting the active value
    /// from the outer scope, otherwise it will return an error.
    pub fn pop_last(&mut self) -> Result<Option<ValueContainer>, InvalidProgramError> {
        // this is only valid if there is exactly one scope in the stack
        if self.stack.len() != 1 {
            return Err(InvalidProgramError::InvalidScopeClose);
        }
        Ok(self.stack.pop().unwrap().active_value)
    }
    
    /// Adds a new scope to the stack.
    pub fn create_scope(&mut self, scope_type: ScopeType) {
        self.stack.push(Scope::default());
    }
    
    /// Sets the active value of the current scope.
    pub fn set_active_value(&mut self, value: ValueContainer) {
        let scope = self.get_current_scope_mut();
        scope.active_value = value.into();
    }
    
    /// Sets the active value of the current scope to None.
    pub fn get_active_value(&self) -> &Option<ValueContainer> {
        let scope = self.get_current_scope();
        &scope.active_value
    }

    /// Returns a mutable reference to the active value of the current scope.
    pub fn get_active_value_mut(&mut self) -> &mut Option<ValueContainer> {
        let scope = self.get_current_scope_mut();
        &mut scope.active_value
    }
    
    /// Clears the active value of the current scope.
    pub fn clear_active_value(&mut self) {
        let scope = self.get_current_scope_mut();
        scope.active_value = None;
    }
    
    /// Sets the active operation for the current scope.
    pub fn set_active_operation(&mut self, operation: Instruction) {
        self.active_operation = Some(operation);
    }
    
    /// Returns the active operation for the current scope, if any.
    pub fn get_active_operation(&self) -> Option<&Instruction> {
        self.active_operation.as_ref()
    }
}
