use crate::datex_values::value_container::ValueContainer;
use crate::global::protocol_structures::instructions::Instruction;

#[derive(Debug, Clone, Default)]
pub struct Scope {
    active_value: ValueContainer,
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
        
    #[inline]
    pub fn get_current_scope_mut(&mut self) -> &mut Scope {
        // assumes that the stack always has at least one scope
        self.stack.last_mut().unwrap()
    }
    // TODO: dont unwrap here! bytecode (scope start/end matching) might be invalid, leading to panic!
    pub fn get_current_scope(&self) -> &Scope {
        // assumes that the stack always has at least one scope
        self.stack.last().unwrap()
    }
    
    pub fn pop(&mut self) -> ValueContainer {
        // assumes that the stack always has at least one scope
        self.stack.pop().unwrap().active_value
    }
    
    pub fn create_scope(&mut self) {
        self.stack.push(Scope::default());
    }
    
    pub fn set_active_value(&mut self, value: ValueContainer) {
        let scope = self.get_current_scope_mut();
        scope.active_value = value;
    }
    
    pub fn get_active_value(&self) -> &ValueContainer {
        let scope = self.get_current_scope();
        &scope.active_value
    }

    pub fn get_active_value_mut(&mut self) -> &mut ValueContainer {
        let scope = self.get_current_scope_mut();
        &mut scope.active_value
    }
    
    pub fn clear_active_value(&mut self) {
        let scope = self.get_current_scope_mut();
        scope.active_value = ValueContainer::Void;
    }
    
    pub fn set_active_operation(&mut self, operation: Instruction) {
        self.active_operation = Some(operation);
    }
    
    pub fn get_active_operation(&self) -> Option<&Instruction> {
        self.active_operation.as_ref()
    }
}
