use std::fmt::Display;
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

// TODO: do we still need ActiveValue if it is just an alias for an Option<ValueContainer>?

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveValue {
    #[default]
    None,
    ValueContainer(ValueContainer),
    //KeyValuePair(Option<ValueContainer>, Option<ValueContainer>),
}

impl From<Option<ValueContainer>> for ActiveValue {
    fn from(value: Option<ValueContainer>) -> Self {
        match value {
            Some(v) => ActiveValue::ValueContainer(v),
            None => ActiveValue::None,
        }
    }
}


impl<T: Into<ValueContainer>> From<T> for ActiveValue {
    fn from(value: T) -> Self {
        ActiveValue::ValueContainer(value.into())
    }
}

impl Display for ActiveValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActiveValue::None => write!(f, "None"),
            ActiveValue::ValueContainer(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scope {
    scope_type: ScopeType,
    active_value: ActiveValue,
    active_operation: Option<Instruction>,
    active_key: Option<ActiveValue>,
    active_slot: Option<u32>,
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
}

impl Default for ScopeStack {
    fn default() -> Self {
        ScopeStack {
            stack: vec![Scope::default()],
        }
    }
}

impl Display for ScopeStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ScopeStack: [")?;
        for scope in self.stack.iter() {
            writeln!(
                f,
                "  [TYPE: {:?}, ACTIVE_VALUE: {}, ACTIVE_OPERATION: {}]",
                scope.scope_type,
                scope.active_value,
                scope.active_operation.clone().map(|op| op.to_string()).unwrap_or("None".to_string())
            )?;
        }
        write!(f, "]")
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

    /// Returns the type of the currently active scope.
    pub fn get_current_scope_type(&self) -> ScopeType {
        self.get_current_scope().scope_type.clone()
    }

    /// Pops the currently active scope from the stack and return its active value
    /// If there is no active value, it returns None
    /// If there are not at least two scopes in the stack, it returns an error
    pub fn pop(&mut self) -> Result<ActiveValue, InvalidProgramError> {
        // make sure there are at least two scopes in the stack, otherwise the byte code was invalid
        if self.stack.len() < 2 {
            return Err(InvalidProgramError::InvalidScopeClose);
        }
        Ok(self.stack.pop().unwrap().active_value)
    }

    /// Pops the last scope from the stack and return its active value.
    /// This should only be called at the end of an execution, when extracting the active value
    /// from the outer scope, otherwise it will return an error.
    pub fn pop_last(&mut self) -> Result<ActiveValue, InvalidProgramError> {
        // this is only valid if there is exactly one scope in the stack
        if self.stack.len() != 1 {
            return Err(InvalidProgramError::InvalidScopeClose);
        }
        Ok(self.stack.pop().unwrap().active_value)
    }

    /// Adds a new scope to the stack.
    pub fn create_scope(&mut self, scope_type: ScopeType) {
        self.stack.push(Scope::new(scope_type));
    }

    /// Sets the active value of the current scope.
    pub fn set_active_value(&mut self, value: ActiveValue) {
        let scope = self.get_current_scope_mut();
        scope.active_value = value;
    }
    
    /// Sets the active value of the current scope to a key-value pair.
    pub fn set_active_key(&mut self, key: ActiveValue) {
        let scope = self.get_current_scope_mut();
        scope.active_key = Some(key);
    }
    
    /// Returns the active key-value pair of the current scope, if any.
    pub fn get_active_key(&mut self) -> Option<ActiveValue> {
        let scope = self.get_current_scope_mut();
        scope.active_key.take()
    }
    
    /// Sets the active value container of the current scope.
    pub fn set_active_value_container(&mut self, value: ValueContainer) {
        let scope = self.get_current_scope_mut();
        scope.active_value = value.into();
    }

    /// Sets the active value of the current scope to None.
    pub fn get_active_value(&self) -> &ActiveValue {
        let scope = self.get_current_scope();
        &scope.active_value
    }

    /// Returns a mutable reference to the active value of the current scope.
    pub fn get_active_value_mut(&mut self) -> &mut ActiveValue {
        let scope = self.get_current_scope_mut();
        &mut scope.active_value
    }

    /// Clears the active value of the current scope.
    pub fn clear_active_value(&mut self) -> ActiveValue {
        let scope = self.get_current_scope_mut();
        // TODO: no clone here
        let active = scope.active_value.clone();
        scope.active_value = ActiveValue::None;
        active
    }

    /// Sets the active operation for the current scope.
    pub fn set_active_operation(&mut self, operation: Instruction) {
        self.get_current_scope_mut().active_operation = Some(operation);
    }

    /// Returns the active operation for the current scope, if any.
    pub fn get_active_operation(&self) -> Option<&Instruction> {
        self.get_current_scope().active_operation.as_ref()
    }


    /// Sets the active slot that is currently been written to.
    pub fn set_active_slot(&mut self, slot: u32) {
        let scope = self.get_current_scope_mut();
        scope.active_slot = Some(slot);
    }

    /// Returns the active slot that is currently been written to, if any.
    pub fn get_active_slot(&self) -> Option<u32> {
        self.get_current_scope().active_slot
    }


    /// Clears the active slot of the current scope.
    pub fn clear_active_slot(&mut self) {
        let scope = self.get_current_scope_mut();
        scope.active_slot = None;
    }
}
