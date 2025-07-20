use crate::compiler::ast_parser::{BinaryOperator, UnaryOperator};
use crate::runtime::execution::InvalidProgramError;
use crate::values::value_container::ValueContainer;
use std::fmt::Display;

#[derive(Debug, Clone, Default)]
pub struct ScopeContainer {
    pub active_value: Option<ValueContainer>,
    pub scope: Scope,
}

#[derive(Debug, Clone, Default)]
pub enum Scope {
    #[default]
    Default,
    Collection,
    RemoteExecution,
    BinaryOperation {
        operator: BinaryOperator,
    },
    UnaryOperation {
        operator: UnaryOperator,
    },
    KeyValuePair,
    SlotAssignment {
        address: u32,
    },
}

impl ScopeContainer {
    pub fn new(scope: Scope) -> Self {
        ScopeContainer {
            active_value: None,
            scope,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopeStack {
    stack: Vec<ScopeContainer>,
}

impl Default for ScopeStack {
    fn default() -> Self {
        ScopeStack {
            stack: vec![ScopeContainer::default()],
        }
    }
}

impl Display for ScopeStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ScopeStack: [")?;
        for scope in self.stack.iter() {
            writeln!(f, "{scope:?}")?;
        }
        write!(f, "]")
    }
}

impl ScopeStack {
    /// Returns a reference to the currently active scope.
    #[inline]
    pub fn get_current_scope(&self) -> &ScopeContainer {
        // assumes that the stack always has at least one scope
        self.stack.last().unwrap()
    }

    /// Returns a mutable reference to the currently active scope.
    #[inline]
    pub fn get_current_scope_mut(&mut self) -> &mut ScopeContainer {
        // assumes that the stack always has at least one scope
        self.stack.last_mut().unwrap()
    }

    /// Pops the currently active scope from the stack and return its active value
    /// If there is no active value, it returns None
    /// If there are not at least two scopes in the stack, it returns an error
    pub fn pop(
        &mut self,
    ) -> Result<Option<ValueContainer>, InvalidProgramError> {
        // make sure there are at least two scopes in the stack, otherwise the byte code was invalid
        if self.stack.len() < 2 {
            return Err(InvalidProgramError::InvalidScopeClose);
        }
        // pop the current scope
        let mut scope = self.stack.pop().unwrap();
        // return active_value if exists
        Ok(scope.active_value.take())
    }

    /// Pops the active value from the current scope, without popping the scope itself.
    pub fn pop_active_value(&mut self) -> Option<ValueContainer> {
        let scope = self.get_current_scope_mut();
        scope.active_value.take()
    }

    /// Adds a new scope to the stack.
    pub fn create_scope(&mut self, scope: Scope) {
        self.stack.push(ScopeContainer::new(scope));
    }

    /// Adds a new scope to the stack with an active container.
    pub fn create_scope_with_active_value(
        &mut self,
        scope: Scope,
        active_value: ValueContainer,
    ) {
        self.stack.push(ScopeContainer {
            active_value: Some(active_value),
            scope,
        });
    }

    /// Sets the active value container of the current scope.
    pub fn set_active_value_container(&mut self, value: ValueContainer) {
        let scope = self.get_current_scope_mut();
        scope.active_value = value.into();
    }

    /// Returns a mutable reference to the active value of the current scope.
    pub fn get_active_value_mut(&mut self) -> &mut Option<ValueContainer> {
        let scope = self.get_current_scope_mut();
        &mut scope.active_value
    }
}
